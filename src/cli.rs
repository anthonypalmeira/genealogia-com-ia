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
use clap::{Args, Parser, Subcommand, ValueEnum};
use serde::Serialize;

use crate::{
    error::Error,
    familysearch::{PersonId, SourceId},
    internal::RouteId,
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
  7  ponte do navegador sem aba ligada: rode `garimpo ponte abrir` e avalie o trecho
     na aba logada
  8  rotas internas desligadas: pare; só o usuário religa

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
        env = "GARIMPO_FS_SITE",
        default_value = "https://www.familysearch.org",
        value_name = "URL",
        help = "URL-base do site do FamilySearch, onde ficam as rotas internas"
    )]
    pub fs_site: String,

    #[arg(
        long = "rotas-internas",
        global = true,
        env = "GARIMPO_ROTAS_INTERNAS",
        value_enum,
        default_value_t = InternalRoutes::On,
        value_name = "ESTADO",
        help = "Liga ou desliga as rotas internas do FamilySearch (o grupo `busca`)",
        long_help = "Liga ou desliga as rotas internas do FamilySearch (o grupo `busca`).\n\n\
                     As rotas internas são as que o site usa e o FamilySearch não documenta \
                     para terceiros. Com `desligadas`, o garimpo só usa as rotas oficiais e o \
                     grupo `busca` sai com 8 sem chamar o FamilySearch."
    )]
    pub internal_routes: InternalRoutes,

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

/// Whether the internal FamilySearch routes may be used.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum InternalRoutes {
    #[value(name = "ligadas")]
    On,
    #[value(name = "desligadas")]
    Off,
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
    #[command(
        name = "busca",
        subcommand,
        about = "Buscas no FamilySearch pelas rotas internas: texto completo, registros, \
                 catálogo e waypoints",
        long_about = "Buscas no FamilySearch pelas rotas internas: texto completo, registros, \
                      catálogo e waypoints.\n\nToda busca entra sozinha no registro de \
                      cobertura (--cobertura), inclusive a que não acha nada (`negativo`) e a \
                      que é bloqueada ou falha (`pendente-de-acesso`, nunca `negativo`). Antes de buscar, \
                      pergunte a `garimpo cobertura consultar` se aquilo já foi buscado.\n\n\
                      As rotas do site recusam (403) fora do navegador. Com a ponte aberta \
                      (`garimpo ponte abrir`), os mesmos comandos passam pela aba logada; sem \
                      ela, vão direto com a sessão entregue. O total de cada busca sai na saída \
                      de erro."
    )]
    Search(SearchCommand),
    #[command(
        name = "cobertura",
        subcommand,
        about = "O registro de cobertura: o que já foi buscado e com que resultado"
    )]
    Coverage(CoverageCommand),
    #[command(
        name = "ponte",
        subcommand,
        about = "A ponte do navegador: as rotas internas passam pela aba logada",
        long_about = "A ponte do navegador: as rotas internas passam pela aba logada.\n\nUm \
                      processo local fica à espera em 127.0.0.1 e a aba logada do FamilySearch \
                      executa os pedidos do grupo `busca`. O token não sai da aba. Uso:\n  1. \
                      `garimpo ponte abrir` imprime um trecho de JavaScript;\n  2. avalie o \
                      trecho uma vez na aba logada em www.familysearch.org;\n  3. confira com \
                      `garimpo ponte estado` e use o grupo `busca` normalmente.\nA aba deve \
                      continuar aberta nessa página: se ela navegar ou fechar, avalie o trecho \
                      de novo."
    )]
    Bridge(BridgeCommand),
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

#[derive(Debug, Subcommand)]
pub enum BridgeCommand {
    #[command(
        name = "abrir",
        about = "Abre a ponte e imprime o trecho de JavaScript a avaliar na aba logada",
        long_about = "Abre a ponte e imprime o trecho de JavaScript a avaliar na aba logada.\n\n\
                      Uma ponte que já estava aberta é fechada antes. O trecho não espera nada: \
                      devolve `ligada` na hora e segue atendendo em segundo plano, sem \
                      temporizadores. Com --json, a linha traz `endereco` e `trecho`."
    )]
    Open,
    #[command(
        name = "estado",
        about = "Confere se há uma aba ligada à ponte. Sai com 7 se não há"
    )]
    Status,
    #[command(name = "fechar", about = "Fecha a ponte")]
    Close,
    /// The bridge process, started by `ponte abrir`.
    #[command(name = "servir", hide = true)]
    Serve,
}

#[derive(Debug, Subcommand)]
pub enum CoverageCommand {
    #[command(
        name = "consultar",
        about = "Responde se algo já foi buscado e com que resultado",
        long_about = "Responde se algo já foi buscado e com que resultado.\n\nMostra as buscas \
                      registradas que contêm todos os termos na consulta, no lugar ou no grupo, \
                      sem diferenciar acentos nem maiúsculas. Sem termos, mostra todas. Linha: \
                      resultado, quando, busca, consulta, lugar, anos, itens vistos, total. \
                      Nenhuma linha quer dizer: nunca buscado.\n\nResultados: `lido` (tudo \
                      visto), `parcial` (há mais do que foi visto: a busca continua aberta), \
                      `negativo` (a busca não achou nada) e `pendente-de-acesso` (bloqueio ou \
                      falha, com o motivo: não diz nada sobre o dado existir)."
    )]
    Query {
        #[arg(value_name = "TERMO", help = "O que procurar no registro de cobertura")]
        terms: Vec<String>,
        #[command(flatten)]
        coverage: CoverageArgs,
    },
}

/// Where a search is recorded.
#[derive(Debug, Args)]
pub struct CoverageArgs {
    #[arg(
        long = "cobertura",
        default_value = "cobertura.jsonl",
        value_name = "ARQUIVO",
        help = "O registro de cobertura da pesquisa, uma linha de JSON por busca"
    )]
    pub log: PathBuf,
}

/// Which page of results a search asks for.
#[derive(Debug, Args)]
pub struct PageArgs {
    #[arg(
        long = "quantos",
        default_value_t = 20,
        value_parser = clap::value_parser!(u32).range(1..=100),
        value_name = "N",
        help = "Quantos resultados pedir, até 100"
    )]
    pub count: u32,
    #[arg(
        long = "inicio",
        default_value_t = 0,
        value_name = "N",
        help = "A partir de qual resultado, contando do 0"
    )]
    pub offset: u32,
}

#[derive(Debug, Subcommand)]
pub enum SearchCommand {
    #[command(
        name = "texto",
        about = "Busca no texto completo (a transcrição automática das imagens)",
        long_about = "Busca no texto completo (a transcrição automática das imagens).\n\n\
                      Sintaxe de TEXTO: `+termo` obriga, `-termo` exclui, `?` e `*` são \
                      curingas. Uma frase entre aspas sem `+` é OPCIONAL: para obrigar um nome \
                      completo use --frase, que escreve `+\"nome completo\"` e tira os acentos. \
                      `+termo` sem --lugar nem --grupo devolve milhares de páginas.\n\nLinha: \
                      ARK da imagem, título, coleção, trecho em volta do termo achado. A \
                      transcrição erra nomes e datas, e nem tudo tem texto: ausência aqui não \
                      prova nada. Leia a imagem."
    )]
    FullText(FullTextArgs),
    #[command(
        name = "grupo",
        about = "Puxa a transcrição de um grupo de imagens inteiro para um arquivo, com retomada",
        long_about = "Puxa a transcrição de um grupo de imagens inteiro para um arquivo, com \
                      retomada.\n\nPagina sem repetir páginas e para no total informado. Grava \
                      uma imagem por linha (JSON com `id`, `titulo`, `data`, `lugar`, `texto`) \
                      em --saida. Se o arquivo já existe, continua de onde parou: depois de um \
                      bloqueio ou do teto da sessão, repita o mesmo comando. Linha de saída: \
                      imagens no arquivo, total do grupo, arquivo. Depois procure os nomes com \
                      `garimpo busca nomes`."
    )]
    Group {
        #[arg(
            value_name = "GRUPO",
            help = "O número do grupo de imagens (filme digital)"
        )]
        group: String,
        #[arg(
            long = "saida",
            value_name = "ARQUIVO",
            help = "O arquivo do grupo puxado"
        )]
        output: PathBuf,
        #[command(flatten)]
        coverage: CoverageArgs,
    },
    #[command(
        name = "nomes",
        about = "Procura uma lista de nomes num grupo puxado, sem chamar o FamilySearch",
        long_about = "Procura uma lista de nomes num grupo puxado, sem chamar o FamilySearch.\n\n\
                      Compara sem diferenciar acentos, maiúsculas nem quebras de linha. Serve \
                      também para palavras de parentesco: `sogro`, `genro`, `herdeiros`. Linha: \
                      nome, ARK da imagem, título, trecho. O nome sem ocorrência não imprime \
                      linha: sai um aviso na saída de erro e a cobertura registra `negativo` \
                      (ou `parcial`, se o grupo não foi puxado inteiro)."
    )]
    Names {
        #[arg(
            value_name = "ARQUIVO",
            help = "O arquivo gravado por `garimpo busca grupo`"
        )]
        file: PathBuf,
        #[arg(value_name = "NOME", required = true, help = "Os nomes a procurar")]
        names: Vec<String>,
        #[command(flatten)]
        coverage: CoverageArgs,
    },
    #[command(
        name = "registros",
        about = "Busca nos registros indexados (pessoas)",
        long_about = "Busca nos registros indexados (pessoas).\n\nA busca é difusa: sem \
                      --exigir, datas e lugares só ordenam os resultados. A entrada devolvida é \
                      a pessoa que casou com a busca, que pode ser o pai e não o registrado. O \
                      índice erra e omite: trate como hipótese e leia a imagem. Linha: ARK do \
                      registro, nome, evento, data, lugar, coleção, outras pessoas do registro."
    )]
    Records(RecordsArgs),
    #[command(
        name = "catalogo",
        about = "Busca no catálogo (filmes e livros). Linha: id do item, título, autores",
        long_about = "Busca no catálogo (filmes e livros). Linha: id do item, título, autores.\n\n\
                      Use para saber se algo foi filmado. Estar no catálogo não garante acesso \
                      às imagens, e há títulos errados. Veja os filmes de um item com `garimpo \
                      busca catalogo-item`."
    )]
    Catalog(CatalogArgs),
    #[command(
        name = "catalogo-item",
        about = "Um item do catálogo com os seus filmes",
        long_about = "Um item do catálogo com os seus filmes.\n\nPrimeira linha: `item`, id, \
                      título, datas, formato. Depois uma linha por filme: `filme`, grupo de \
                      imagens, microfilme, acesso (`aberto`, `restrito` ou o código do \
                      FamilySearch), conteúdo. O grupo de imagens serve em `garimpo busca \
                      grupo`."
    )]
    CatalogItem {
        #[arg(
            value_name = "ID",
            help = "O id do item, como saiu de `garimpo busca catalogo`"
        )]
        id: u64,
        #[command(flatten)]
        coverage: CoverageArgs,
    },
    #[command(
        name = "waypoints",
        about = "Percorre uma coleção sem índice: localidades, volumes e imagens em ordem",
        long_about = "Percorre uma coleção sem índice: localidades, volumes e imagens em \
                      ordem.\n\nSó com a coleção, lista o primeiro nível. Com um WAYPOINT, \
                      desce nele. Linha de nível: `waypoint`, id para descer, título. Linha de \
                      imagem: `imagem`, posição no volume, ARK."
    )]
    Waypoints {
        #[arg(value_name = "COLECAO", help = "O id da coleção")]
        collection: RouteId,
        #[arg(value_name = "WAYPOINT", help = "O id do nível em que descer")]
        waypoint: Option<RouteId>,
        #[arg(
            long = "quantos",
            default_value_t = 100,
            value_parser = clap::value_parser!(u32).range(1..=1000),
            value_name = "N",
            help = "Quantos filhos pedir, até 1000"
        )]
        count: u32,
        #[arg(
            long = "inicio",
            default_value_t = 0,
            value_name = "N",
            help = "A partir de qual filho, contando do 0"
        )]
        offset: u32,
        #[command(flatten)]
        coverage: CoverageArgs,
    },
}

#[derive(Debug, Args)]
pub struct FullTextArgs {
    #[arg(
        value_name = "TEXTO",
        help = "A consulta, na sintaxe do texto completo"
    )]
    pub text: Option<String>,
    #[arg(
        long = "frase",
        value_name = "FRASE",
        help = "Uma frase obrigatória, como um nome completo. Pode repetir"
    )]
    pub phrases: Vec<String>,
    #[arg(
        long = "nome",
        value_name = "NOME",
        help = "Procura só entre os nomes reconhecidos na página"
    )]
    pub full_name: Option<String>,
    #[arg(long = "lugar", value_name = "LUGAR", help = "O lugar do documento")]
    pub place: Option<String>,
    #[arg(long = "de", value_name = "ANO", help = "O ano inicial do documento")]
    pub from: Option<u16>,
    #[arg(long = "ate", value_name = "ANO", help = "O ano final do documento")]
    pub to: Option<u16>,
    #[arg(
        long = "grupo",
        value_name = "GRUPO",
        help = "Busca só dentro de um grupo de imagens (filme digital)"
    )]
    pub group: Option<String>,
    #[command(flatten)]
    pub page: PageArgs,
    #[command(flatten)]
    pub coverage: CoverageArgs,
}

#[derive(Debug, Args)]
pub struct RecordsArgs {
    #[arg(long = "nome", value_name = "NOME", help = "O prenome")]
    pub given_name: Option<String>,
    #[arg(long = "sobrenome", value_name = "SOBRENOME", help = "O sobrenome")]
    pub surname: Option<String>,
    #[arg(long = "pai", value_name = "NOME", help = "O prenome do pai")]
    pub father: Option<String>,
    #[arg(long = "mae", value_name = "NOME", help = "O prenome da mãe")]
    pub mother: Option<String>,
    #[arg(long = "conjuge", value_name = "NOME", help = "O prenome do cônjuge")]
    pub spouse: Option<String>,
    #[arg(
        long = "lugar",
        value_name = "LUGAR",
        help = "Um lugar de qualquer evento"
    )]
    pub place: Option<String>,
    #[arg(
        long = "de",
        value_name = "ANO",
        help = "O ano inicial do nascimento ou batismo"
    )]
    pub from: Option<u16>,
    #[arg(
        long = "ate",
        value_name = "ANO",
        help = "O ano final do nascimento ou batismo"
    )]
    pub to: Option<u16>,
    #[arg(long = "colecao", value_name = "ID", help = "Busca só numa coleção")]
    pub collection: Option<String>,
    #[arg(
        long = "exigir",
        help = "Torna todos os termos obrigatórios, em vez de só ordenar por eles"
    )]
    pub require: bool,
    #[command(flatten)]
    pub page: PageArgs,
    #[command(flatten)]
    pub coverage: CoverageArgs,
}

#[derive(Debug, Args)]
pub struct CatalogArgs {
    #[arg(
        long = "lugar",
        value_name = "LUGAR",
        help = "O lugar, do país à cidade: `Brasil, Minas Gerais, Mariana`"
    )]
    pub place: Option<String>,
    #[arg(long = "titulo", value_name = "TITULO", help = "Palavras do título")]
    pub title: Option<String>,
    #[arg(long = "palavras", value_name = "PALAVRAS", help = "Palavras-chave")]
    pub keywords: Option<String>,
    #[command(flatten)]
    pub page: PageArgs,
    #[command(flatten)]
    pub coverage: CoverageArgs,
}
