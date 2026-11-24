//! The arguments and environment variables of `garimpo`, already typed.
//!
//! Command names, flag names and help text are the Portuguese surface of the
//! CLI, so they are spelled out as strings instead of derived from the code.

use std::{
    env, fs,
    os::unix::fs::DirBuilderExt,
    path::{Path, PathBuf},
};

use anyhow::Context;
use clap::{Args, Parser, Subcommand};
use serde::Serialize;

use crate::{
    error::Error,
    familysearch::{PersonId, SourceId},
};

const ABOUT: &str = "\
CLI para agentes de IA pesquisarem genealogia no FamilySearch sem bloquear a conta.

Feita para agentes, não para pessoas:
  - nada é interativo; tudo vem por argumento, variável de ambiente ou entrada padrão;
  - a saída é curta e estável, uma linha por item, com os campos separados por TAB
    (campo vazio é `-`); com --json, cada item é uma linha de JSON;
  - avisos e erros vão para a saída de erro; cada erro traz o próximo passo.

Comece por `garimpo sessao entregar` e confira com `garimpo sessao conferir`.";

const EXIT_CODES: &str = "\
Códigos de saída:
  0  deu certo
  1  falha inesperada (rede, disco, resposta estranha): leia a causa e rode de novo
  2  entrada inválida: corrija os argumentos
  3  login necessário (sessão ausente ou expirada): peça ao usuário para entrar no
     FamilySearch, entregue a sessão nova e rode de novo
  4  bloqueio (403, 429 repetido ou pausa longa): pare, ou espere o que o erro pedir;
     não é \"não existe\"
  5  teto de requisições da sessão atingido: pare; a decisão é do usuário
  6  escrita sem rodada autorizada: peça a aprovação do usuário

Termos do FamilySearch: uso pessoal e não comercial. O garimpo não se disfarça de
navegador, não guarda o token em disco e para no teto de requisições da sessão.";

/// The whole command line.
#[derive(Debug, Parser)]
#[command(name = "garimpo", version, about = ABOUT, after_help = EXIT_CODES)]
pub struct Cli {
    #[command(flatten)]
    pub config: Config,
    #[command(subcommand)]
    pub command: Command,
}

/// What applies to every command.
#[derive(Debug, Args)]
#[command(next_help_heading = "Opções gerais")]
pub struct Config {
    #[arg(
        long,
        global = true,
        help = "Saída em JSON: uma linha de JSON por item"
    )]
    pub json: bool,

    #[arg(
        long,
        global = true,
        env = "GARIMPO_DIR",
        default_value_os_t = default_dir(),
        hide_default_value = true,
        value_name = "PASTA",
        help = "Pasta de estado da conta, uma por conta do FamilySearch. Padrão: ~/.garimpo",
        long_help = "Pasta de estado da conta, uma por conta do FamilySearch. Padrão: \
                     ~/.garimpo\n\nGuarda a trava e o ritmo compartilhados entre processos e \
                     o soquete da sessão."
    )]
    pub dir: PathBuf,

    #[arg(
        long,
        global = true,
        env = "GARIMPO_FS_API",
        default_value = "https://api.familysearch.org",
        value_name = "URL",
        help = "URL-base da API do FamilySearch"
    )]
    pub fs_api: String,

    #[arg(
        long,
        global = true,
        env = "GARIMPO_FS_IDENT",
        default_value = "https://ident.familysearch.org",
        value_name = "URL",
        help = "URL-base do login do FamilySearch (OAuth)"
    )]
    pub fs_ident: String,

    #[arg(
        long = "teto-sessao",
        global = true,
        env = "GARIMPO_TETO_SESSAO",
        default_value_t = 2000,
        value_name = "N",
        help = "Teto de requisições por sessão entregue. Só o usuário decide aumentar"
    )]
    pub session_cap: u32,

    #[arg(
        long = "orcamento-ms",
        global = true,
        env = "GARIMPO_ORCAMENTO_MS",
        default_value_t = 12_000,
        value_name = "MS",
        help = "Orçamento de tempo de processamento do FamilySearch por janela, em \
                milissegundos"
    )]
    pub budget_ms: u64,

    #[arg(
        long = "janela-s",
        global = true,
        env = "GARIMPO_JANELA_S",
        default_value_t = 60,
        value_name = "S",
        help = "Tamanho da janela do orçamento, em segundos"
    )]
    pub window_s: u64,

    #[arg(
        long = "espera-max-s",
        global = true,
        env = "GARIMPO_ESPERA_MAX_S",
        default_value_t = 900,
        value_name = "S",
        help = "Espera máxima por uma pausa do FamilySearch, em segundos",
        long_help = "Espera máxima por uma pausa do FamilySearch, em segundos.\n\nO garimpo \
                     espera sozinho as pausas até esse tamanho. Acima dele, sai com 4 e diz \
                     quanto falta; a pausa continua valendo para os próximos comandos."
    )]
    pub max_wait_s: u64,

    #[arg(
        long = "pausa-429-s",
        global = true,
        env = "GARIMPO_PAUSA_429_S",
        default_value_t = 60,
        value_name = "S",
        help = "Pausa mínima depois de um 429, em segundos",
        long_help = "Pausa mínima depois de um 429, em segundos.\n\nVale o maior entre ela e o \
                     `Retry-After` do servidor."
    )]
    pub pause_429_s: u64,
}

fn default_dir() -> PathBuf {
    env::var_os("HOME").map_or_else(
        || ".garimpo".into(),
        |home| PathBuf::from(home).join(".garimpo"),
    )
}

impl Config {
    /// The state directory, created on first use and closed to other users.
    ///
    /// # Errors
    /// `Error::Other` if the directory cannot be created.
    pub fn state_dir(&self) -> Result<&Path, Error> {
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(&self.dir)
            .with_context(|| format!("não consegui criar a pasta {}", self.dir.display()))?;
        Ok(&self.dir)
    }

    /// Prints one item: the short line or, with `--json`, its JSON on one line.
    ///
    /// # Errors
    /// `Error::Other` if the item cannot be turned into JSON.
    pub fn print(&self, line: &str, item: &impl Serialize) -> Result<(), Error> {
        if self.json {
            let json = serde_json::to_string(item).context("não consegui gerar o JSON da saída")?;
            println!("{json}");
        } else {
            println!("{line}");
        }
        Ok(())
    }
}

#[derive(Debug, Subcommand)]
pub enum Command {
    #[command(
        name = "sessao",
        subcommand,
        about = "A sessão do FamilySearch: entregar, conferir e encerrar"
    )]
    Session(SessionCommand),
    #[command(
        name = "fs",
        subcommand,
        about = "A árvore do FamilySearch pelas rotas oficiais: ler, percorrer e gravar"
    )]
    Fs(FsCommand),
}

#[derive(Debug, Subcommand)]
pub enum SessionCommand {
    #[command(
        name = "entregar",
        about = "Guarda a sessão do navegador logado, lida da entrada padrão",
        long_about = "Guarda a sessão do navegador logado, lida da entrada padrão.\n\nO token é \
                      o valor do cookie `fssessionid` da aba logada. Entregue-o uma vez por \
                      login: `printf %s \"$TOKEN\" | garimpo sessao entregar`. Ele fica só na \
                      memória de um processo guardião e nunca vai para o disco. Uma entrega \
                      nova troca a sessão e zera a contagem do teto."
    )]
    Deliver,
    #[command(
        name = "oauth",
        about = "Faz o login pela API oficial (OAuth com PKCE), para quem tem chave de app",
        long_about = "Faz o login pela API oficial (OAuth com PKCE), para quem tem chave de \
                      app.\n\nImprime na primeira linha a URL de autorização: mostre-a ao \
                      usuário para abrir no navegador. Depois espera o retorno em \
                      http://127.0.0.1:PORTA, que é o retorno a registrar no FamilySearch, e \
                      guarda a sessão como `sessao entregar`."
    )]
    Oauth {
        #[arg(
            long = "chave",
            env = "GARIMPO_FS_CHAVE",
            value_name = "CHAVE",
            help = "A chave de app (client_id) do FamilySearch"
        )]
        app_key: String,
        #[arg(
            long = "porta",
            default_value_t = 8537,
            value_name = "PORTA",
            help = "A porta do retorno local"
        )]
        port: u16,
        #[arg(
            long = "espera",
            default_value_t = 300,
            value_name = "S",
            help = "Quantos segundos esperar pelo login. Sai com 3 se o usuário não entrar"
        )]
        wait_s: u64,
    },
    #[command(
        name = "conferir",
        about = "Confere no FamilySearch se a sessão guardada ainda vale. Sai com 3 se não vale",
        long_about = "Confere no FamilySearch se a sessão guardada ainda vale. Sai com 3 se não \
                      vale.\n\nRode antes de um trabalho longo."
    )]
    Check,
    #[command(name = "encerrar", about = "Descarta a sessão guardada")]
    End,
    /// The session holder process, started by `sessao entregar`.
    #[command(name = "guardar", hide = true)]
    Hold,
}

#[derive(Debug, Subcommand)]
pub enum FsCommand {
    #[command(
        name = "pessoa",
        about = "Uma pessoa, numa linha",
        long_about = "Uma pessoa, numa linha.\n\nLinha: id, nome, sexo, data e lugar de \
                      nascimento, data e lugar de morte. Trate tudo como hipótese: a árvore é \
                      editada por qualquer usuário."
    )]
    Person {
        #[arg(value_name = "ID")]
        id: PersonId,
    },
    #[command(
        name = "pais",
        about = "Os pais de uma pessoa, uma linha por pessoa (como em `pessoa`)"
    )]
    Parents {
        #[arg(value_name = "ID")]
        id: PersonId,
    },
    #[command(
        name = "filhos",
        about = "Os filhos de uma pessoa, uma linha por pessoa (como em `pessoa`)"
    )]
    Children {
        #[arg(value_name = "ID")]
        id: PersonId,
    },
    #[command(
        name = "conjuges",
        about = "Os cônjuges de uma pessoa, uma linha por pessoa (como em `pessoa`)"
    )]
    Spouses {
        #[arg(value_name = "ID")]
        id: PersonId,
    },
    #[command(
        name = "fontes",
        about = "As fontes anexadas a uma pessoa. Linha: id da fonte, título, URL"
    )]
    Sources {
        #[arg(value_name = "ID")]
        id: PersonId,
    },
    #[command(
        name = "notas",
        about = "As notas de uma pessoa. Linha: assunto, texto"
    )]
    Notes {
        #[arg(value_name = "ID")]
        id: PersonId,
    },
    #[command(
        name = "ascendencia",
        about = "Sobe pelos pais e grava a ascendência num arquivo, com retomada",
        long_about = "Sobe pelos pais e grava a ascendência num arquivo, com retomada.\n\n\
                      Percorre em largura, em lotes de 200 pessoas, sem corte no número de \
                      pessoas. Grava uma pessoa por linha (JSON) em --saida. Se o arquivo já \
                      existe, continua de onde ele parou: depois de um bloqueio ou do teto da \
                      sessão, repita o mesmo comando. Linha de saída: número de pessoas no \
                      arquivo, arquivo."
    )]
    Ancestry(TraversalArgs),
    #[command(
        name = "descendencia",
        about = "Desce pelos filhos e grava a descendência num arquivo, como `ascendencia`"
    )]
    Descendancy(TraversalArgs),
    #[command(
        name = "criar-fonte",
        about = "Cria uma fonte e a anexa à pessoa. Exige --rodada",
        long_about = "Cria uma fonte e a anexa à pessoa. Exige --rodada.\n\nNão duplica: se a \
                      pessoa já tem uma fonte com a mesma URL, responde `ja-existe` e não grava \
                      nada. Senão cria, anexa, relê e registra. Linha: resultado (`anexada` ou \
                      `ja-existe`), id da fonte, id da pessoa."
    )]
    CreateSource {
        #[command(flatten)]
        write: WriteArgs,
        #[command(flatten)]
        source: NewSource,
    },
    #[command(
        name = "anexar-fonte",
        about = "Anexa à pessoa uma fonte que já existe. Exige --rodada",
        long_about = "Anexa à pessoa uma fonte que já existe. Exige --rodada.\n\nServe para o \
                      documento que cita várias pessoas: crie a fonte na primeira e anexe-a às \
                      outras. Mesmas regras e mesma linha de `criar-fonte`."
    )]
    AttachSource {
        #[command(flatten)]
        write: WriteArgs,
        #[arg(
            long = "fonte",
            value_name = "ID",
            help = "O id da fonte, como saiu de `criar-fonte` ou de `fontes`"
        )]
        source: SourceId,
    },
    #[command(
        name = "criar-nota",
        about = "Cria uma nota na pessoa. Exige --rodada",
        long_about = "Cria uma nota na pessoa. Exige --rodada.\n\nNão duplica uma nota com o \
                      mesmo assunto e texto. Grava, relê e registra. Linha: resultado (`criada` \
                      ou `ja-existe`), id da nota, id da pessoa."
    )]
    CreateNote {
        #[command(flatten)]
        write: WriteArgs,
        #[arg(long = "assunto", value_name = "ASSUNTO", help = "O assunto da nota")]
        subject: String,
        #[arg(long = "texto", value_name = "TEXTO", help = "O texto da nota")]
        text: String,
    },
}

/// What every write to the tree requires. `garimpo` only writes sources and
/// notes: it never changes relationships, merges persons or edits facts.
#[derive(Debug, Args)]
pub struct WriteArgs {
    #[arg(value_name = "ID", help = "A pessoa que recebe a gravação")]
    pub id: PersonId,
    #[arg(
        long = "rodada",
        value_name = "RODADA",
        help = "A rodada de gravação que o usuário autorizou",
        long_help = "A rodada de gravação que o usuário autorizou.\n\nIdentifique a aprovação, \
                     por exemplo com a data e o resumo do plano aprovado. Sem ela a escrita é \
                     recusada (código 6). Não invente: mostre o plano ao usuário e espere o sim."
    )]
    pub round: Option<String>,
    #[arg(
        long = "motivo",
        value_name = "MOTIVO",
        help = "Por que a alteração é feita. Vai para o registro e, nas fontes, para o \
                FamilySearch como mensagem da alteração"
    )]
    pub reason: String,
    #[arg(
        long = "registro",
        default_value = "alteracoes.jsonl",
        value_name = "ARQUIVO",
        help = "O registro de alterações, uma linha de JSON por alteração",
        long_help = "O registro de alterações, uma linha de JSON por alteração.\n\nCada linha \
                     traz `quando`, `acao`, `pessoa`, `fonte` ou `nota`, `motivo`, `rodada` e `conferida`. \
                     Anexar fonte e criar nota deixam duas linhas: uma quando o FamilySearch \
                     aceita (`conferida: false`) e outra quando a releitura confirma \
                     (`conferida: true`). Só a primeira quer dizer: confira no FamilySearch."
    )]
    pub log: PathBuf,
}

/// The data of a new source.
#[derive(Debug, Args)]
pub struct NewSource {
    #[arg(
        long = "titulo",
        value_name = "TITULO",
        help = "O título da fonte, como aparece na lista de fontes da pessoa"
    )]
    pub title: String,
    #[arg(
        long = "citacao",
        value_name = "CITACAO",
        help = "A citação completa do documento"
    )]
    pub citation: String,
    #[arg(long, value_name = "URL", help = "O link do documento")]
    pub url: String,
    #[arg(
        long = "nota",
        value_name = "NOTA",
        help = "A transcrição ou outra nota da fonte"
    )]
    pub note: Option<String>,
}

#[derive(Debug, Args)]
pub struct TraversalArgs {
    #[arg(value_name = "ID", help = "A pessoa de onde a varredura parte")]
    pub id: PersonId,
    #[arg(
        long = "geracoes",
        default_value_t = 8,
        value_name = "N",
        help = "Quantas gerações além da pessoa de partida"
    )]
    pub generations: u32,
    #[arg(
        long = "saida",
        value_name = "ARQUIVO",
        help = "O arquivo da varredura. Cada linha traz `geracao`, `pais`, `filhos` e `pessoa`"
    )]
    pub output: PathBuf,
}
