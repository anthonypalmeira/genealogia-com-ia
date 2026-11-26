//! The arguments and environment variables of `garimpo`, already typed.
//!
//! Command names, flag names and help text are the Portuguese surface of the
//! CLI, so they are spelled out as strings instead of derived from the code.

use std::{
    env, fmt, fs,
    os::unix::fs::DirBuilderExt,
    path::{Path, PathBuf},
    str::FromStr,
};

use anyhow::Context;
use clap::{Args, Parser, Subcommand, ValueEnum, builder::NonEmptyStringValueParser};
use serde::Serialize;

use crate::{
    coverage::Outcome,
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
     ou acesso restrito a um filme ou a uma imagem: anote e siga. Nenhum dos dois é
     \"não existe\"
  5  teto de requisições da sessão atingido: pare; a decisão é do usuário
  6  escrita sem rodada autorizada: peça a aprovação do usuário
  7  ponte do navegador sem aba ligada: rode `garimpo ponte abrir` e avalie o trecho
     na aba logada
  8  rotas internas desligadas: pare; só o usuário religa

Os mesmos códigos valem para o digitarq: 4 é um bloqueio dele.

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
        long = "fs-imagens",
        global = true,
        env = "GARIMPO_FS_IMAGENS",
        default_value = "https://sg30p0.familysearch.org/service/records/storage/dascloud/das/v2",
        value_name = "URL",
        help = "URL-base do armazenamento de imagens do FamilySearch"
    )]
    pub fs_images: String,

    #[arg(
        long,
        global = true,
        env = "GARIMPO_DIGITARQ",
        default_value = "https://digitarq.arquivos.pt",
        value_name = "URL",
        help = "URL-base do digitarq (Torre do Tombo e arquivos distritais)"
    )]
    pub digitarq: String,

    #[arg(
        long = "intervalo-imagens-ms",
        global = true,
        env = "GARIMPO_INTERVALO_IMAGENS_MS",
        default_value_t = 1500,
        value_name = "MS",
        help = "Intervalo mínimo depois de cada imagem em resolução original, em milissegundos",
        long_help = "Intervalo mínimo depois de cada imagem em resolução original, em \
                     milissegundos.\n\nVale para a conta inteira, como as pausas: nenhum \
                     processo garimpo chama o FamilySearch antes dele. O padrão dá cerca de 40 \
                     imagens por minuto; cerca de 50 por minuto já bloquearam uma conta."
    )]
    pub image_interval_ms: u64,

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
    #[command(
        name = "imagens",
        subcommand,
        about = "Imagens de um filme do FamilySearch: listar, baixar, triar e recortar",
        long_about = "Imagens de um filme do FamilySearch: listar, baixar, triar e recortar.\n\n\
                      Ordem de uso: `listar` (quantas imagens o filme tem), `miniaturas` e \
                      `folha` (triagem), `baixar` (resolução original, com manifesto e retomada) \
                      e `recortar` ou `contraste` (leitura). O GRUPO é o grupo de imagens \
                      (filme digital), como sai de `garimpo busca catalogo-item`.\n\nA imagem \
                      de um documento recebe só recorte, ampliação e contraste global: nenhum \
                      comando desenha, apaga ou recompõe escrita, e o original baixado nunca é \
                      alterado. Cada derivado diz no nome a operação que o gerou."
    )]
    Images(ImagesCommand),
    #[command(
        name = "digitarq",
        subcommand,
        about = "O digitarq (Torre do Tombo e arquivos distritais): buscar, ler, baixar e \
                 colher pelo OAI-PMH",
        long_about = "O digitarq (Torre do Tombo e arquivos distritais): buscar, ler, baixar e \
                      colher pelo OAI-PMH.\n\nÉ uma API pública, sem login: não usa a sessão \
                      nem o teto do FamilySearch. O garimpo espera 0,3 s entre os pedidos e não \
                      insiste diante de um bloqueio. Toda consulta entra no registro de \
                      cobertura (--cobertura), e um bloqueio entra como `pendente-de-acesso`."
    )]
    Digitarq(DigitarqCommand),
    #[command(
        name = "diagrama",
        about = "Desenha uma linhagem e marca no desenho os furos que ela tem",
        long_about = "Desenha uma linhagem e marca no desenho os furos que ela tem.\n\n\
                      DESCRICAO é um arquivo JSON (ou `-`, para a entrada padrão):\n  \
                      {\"titulo\": \"…\",\n   \"pessoas\": [{\"id\": \"joaquim\", \"nome\": \"Joaquim …\", \
                      \"nascimento\": 1820, \"morte\": 1880}],\n   \"elos\": [{\"de\": \"joaquim\", \
                      \"para\": \"manoel\", \"nota\": \"B\", \"identidade\": \"A\",\n             \
                      \"documento\": \"Batismo de Joaquim, 1820 [V]\", \"papel\": \"direto\"}]}\n\
                      Cada elo diz que `de` é filho de `para`; com `relacao` (\"irmão\", \
                      \"padrinho\"), diz essa relação. `papel` é `direto` (a linhagem), \
                      `colateral` (parente que ajuda a provar) ou `contexto`. `nota` e \
                      `identidade` vão de A a F, pela régua de prova; `titulo`, as datas, \
                      `relacao` e `identidade` são opcionais.\n\nRegras checadas, com o que \
                      quebra marcado com ⚠ no desenho e listado na saída de erro: todo elo tem \
                      nota e documento; as datas são possíveis; os elos diretos formam uma \
                      linha só, ligada a todas as pessoas. O elo mais fraco da linha direta \
                      sai em destaque. Mostre no máximo ~20 pessoas por desenho.\n\n\
                      A linhagem não informa o sexo. O diagrama usa a regra do pai ou de \
                      sexo desconhecido: 12 a 80 anos no nascimento do filho, que pode nascer \
                      até um ano depois da morte. Em `fs caminhos`, a regra da mãe (sexo \
                      feminino na árvore) marca idade fora de 12 a 55 anos ou nascimento \
                      posterior à morte dela."
    )]
    Diagram(DiagramArgs),
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
                      vale.\n\nMostra o agente da árvore e a pessoa da conta, quando informados."
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
        about = "As fontes anexadas a uma pessoa. Linha: id da fonte, título, URL, agente, data UTC, evento (`criacao` do anexo ou, sem ela, a última `alteracao`), autoria desse agente"
    )]
    Sources {
        #[arg(value_name = "ID")]
        id: PersonId,
    },
    #[command(
        name = "notas",
        about = "As notas de uma pessoa. Linha: assunto, texto, agente, data UTC, evento (`criacao` ou, sem ela, a última `alteracao`), autoria desse agente"
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
                      arquivo, arquivo.\n\nO limite é --geracoes, --ano-corte ou os dois; vale o \
                      que parar primeiro. Repetir o comando com --geracoes maior aprofunda a \
                      mesma varredura; outro --ano-corte pede outro arquivo."
    )]
    Ancestry {
        #[command(flatten)]
        traversal: TraversalArgs,
        #[arg(
            long = "ano-corte",
            value_name = "ANO",
            help = "Para de subir em quem nasceu antes deste ano",
            long_help = "Para de subir em quem nasceu antes deste ano.\n\nQuem nasceu antes do \
                         corte entra no arquivo, mas os pais dele não são pedidos. Sem data de \
                         nascimento, vale a de batismo. A data é a da forma padrão do \
                         FamilySearch; sem forma padrão, vale o primeiro ano do texto digitado, \
                         mesmo que o texto diga um intervalo. Não para a subida quem não tem \
                         nenhuma das duas datas na árvore nem quem as tem como intervalo na \
                         forma padrão (`antes de 1800`). Sem --geracoes, o corte é o único \
                         limite: um ano muito antigo pede a árvore inteira, até o teto da sessão."
        )]
        cut_year: Option<i32>,
    },
    #[command(
        name = "descendencia",
        about = "Desce pelos filhos e grava a descendência num arquivo, como `ascendencia`"
    )]
    Descendancy(TraversalArgs),
    #[command(
        name = "varredura",
        about = "As pessoas de uma varredura, uma por linha, sem chamar o FamilySearch",
        long_about = "As pessoas de uma varredura, uma por linha, sem chamar o FamilySearch.\n\n\
                      Lê o arquivo de `ascendencia` ou de `descendencia`, completo ou não. Serve \
                      para achar alvos por nome, fato ou lugar. Linha: id, geração, todos os \
                      nomes que a árvore dá (separados por ` / `) e depois um campo por fato: \
                      tipo, valor, data e lugar. A data vem na forma padrão do FamilySearch, \
                      quando houver, ou como foi digitada. O tipo vem como no GEDCOM X: `Birth`, \
                      `Death`, `Christening`, `Occupation`. Trate tudo como hipótese."
    )]
    Traversal {
        #[arg(value_name = "ARQUIVO", help = "O arquivo da varredura")]
        file: PathBuf,
    },
    #[command(
        name = "caminhos",
        about = "Todos os caminhos da pessoa de partida até cada alvo, numa varredura de \
                 ascendência, sem chamar o FamilySearch",
        long_about = "Todos os caminhos da pessoa de partida até cada alvo, numa varredura de \
                      ascendência, sem chamar o FamilySearch.\n\nUm ancestral aparece mais de \
                      uma vez na ascendência quando parentes se casaram: cada caminho sai \
                      inteiro, com um número. Linha, uma por elo: número do caminho, alvo, id, \
                      nome e nascimento do filho, id, nome, nascimento e morte do pai ou da \
                      mãe, aviso. Um alvo que não está no arquivo não imprime linha: sai um \
                      aviso na saída de erro. Isso só diz que a varredura não chegou a ele: o \
                      arquivo pode estar incompleto, e a árvore também.\n\nO aviso usa o sexo \
                      registrado na árvore. Para a mãe (sexo feminino), marca idade fora de \
                      12 a 55 anos no nascimento do filho ou nascimento depois da morte dela. \
                      Para o pai ou quando falta o sexo, marca idade fora de 12 a 80 anos ou \
                      nascimento mais de um ano depois da morte. A conta usa o mês e o dia \
                      quando as duas datas os têm na forma padrão do FamilySearch; de uma data \
                      só digitada vale o ano, e um intervalo na forma padrão não conta como \
                      data. O elo continua no caminho, com a marca para conferir as datas e o \
                      parentesco. A árvore só indica os caminhos; a prova de cada elo vem dos documentos."
    )]
    Paths {
        #[arg(
            value_name = "ARQUIVO",
            help = "O arquivo gravado por `garimpo fs ascendencia`"
        )]
        file: PathBuf,
        #[arg(
            value_name = "ALVO",
            required = true,
            help = "Os ids dos ancestrais-alvo"
        )]
        targets: Vec<PersonId>,
    },
    #[command(
        name = "retratos",
        about = "Baixa os retratos das pessoas de uma varredura, para ver o que só a imagem \
                 mostra",
        long_about = "Baixa os retratos das pessoas de uma varredura, para ver o que só a imagem \
                      mostra, como um selo posto sobre o retrato.\n\nFaz um pedido ao \
                      FamilySearch por pessoa do arquivo de `ascendencia` ou de `descendencia`, \
                      no ritmo da conta e dentro do teto da sessão; depois de cada retrato, o \
                      pedido seguinte espera o intervalo das imagens (--intervalo-imagens-ms). \
                      Cada retrato vai para PASTA como `<id>.jpg`, e quem o FamilySearch diz \
                      não ter retrato entra em PASTA/sem-retrato.txt, um id por linha. Repetir \
                      o comando retoma: não pede de novo nem o retrato que está na pasta nem \
                      quem está na lista. Pessoa fundida ou apagada e imagem que não veio \
                      inteira ficam pendentes de acesso, com o motivo na saída de erro, e são \
                      pedidas de novo na próxima vez: não é o mesmo que não ter retrato. \
                      Linha: retratos baixados agora, já havia, pessoas sem retrato, pendentes \
                      de acesso, pasta.\n\nVeja vários de uma vez com `garimpo imagens folha`. \
                      O retrato é escolhido por usuários da árvore: é pista, nunca prova."
    )]
    Portraits {
        #[arg(value_name = "ARQUIVO", help = "O arquivo da varredura")]
        file: PathBuf,
        #[arg(
            long = "pasta",
            default_value = "retratos",
            value_name = "PASTA",
            help = "A pasta dos retratos, com a lista sem-retrato.txt dentro"
        )]
        folder: PathBuf,
    },
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
        value_name = "N",
        help = "Quantas gerações além da pessoa de partida. Padrão: 8; na ascendência com \
                --ano-corte, sem limite"
    )]
    pub generations: Option<u32>,
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
    #[command(
        name = "registrar",
        about = "Registra uma busca feita fora do garimpo: num arquivo, num site ou num livro",
        long_about = "Registra uma busca feita fora do garimpo: num arquivo, num site ou num \
                      livro.\n\nAs buscas do garimpo entram sozinhas no registro; esta é para o \
                      que foi buscado por outro caminho, como o catálogo de um arquivo no \
                      navegador, o índice de um livro ou um pedido respondido. ONDE passa a ser \
                      a `busca` do registro, e `garimpo cobertura consultar` acha a busca por \
                      ele, pela consulta e pelo lugar. Linha: a mesma de `consultar`.\n\nO \
                      resultado é o que você viu, não o que supõe: `negativo` só quando a busca \
                      foi feita e não achou nada; bloqueio, captcha, login e página fora do ar \
                      são `pendente-de-acesso`, com o --motivo."
    )]
    Record {
        #[arg(
            value_name = "ONDE",
            value_parser = NonEmptyStringValueParser::new(),
            help = "Onde a busca foi feita: o arquivo, o site ou o livro"
        )]
        source: String,
        #[arg(
            value_name = "CONSULTA",
            value_parser = NonEmptyStringValueParser::new(),
            help = "O que foi procurado, como foi escrito"
        )]
        query: String,
        #[arg(
            long = "resultado",
            value_enum,
            value_name = "RESULTADO",
            help = "O que veio da busca"
        )]
        outcome: Outcome,
        #[arg(
            long = "lugar",
            value_name = "LUGAR",
            help = "O lugar do que foi procurado"
        )]
        place: Option<String>,
        #[arg(
            long = "de",
            value_name = "ANO",
            help = "O ano inicial do que foi procurado"
        )]
        from: Option<u16>,
        #[arg(
            long = "ate",
            value_name = "ANO",
            help = "O ano final do que foi procurado"
        )]
        to: Option<u16>,
        #[arg(
            long = "vistos",
            default_value_t = 0,
            value_name = "N",
            help = "Quantos itens foram vistos"
        )]
        seen: u64,
        #[arg(
            long = "total",
            value_name = "N",
            help = "Quantos itens a busca achou ao todo, se ela disse"
        )]
        total: Option<u64>,
        #[arg(
            long = "motivo",
            value_name = "MOTIVO",
            required_if_eq("outcome", "pendente-de-acesso"),
            help = "Por que o acesso está pendente. Obrigatório com `pendente-de-acesso`"
        )]
        reason: Option<String>,
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

#[derive(Debug, Subcommand)]
pub enum ImagesCommand {
    #[command(
        name = "listar",
        about = "Diz quantas imagens o filme tem e quais já estão na pasta",
        long_about = "Diz quantas imagens o filme tem e quais já estão na pasta.\n\nGrava o \
                      total no manifesto. As imagens são numeradas de 1 ao total, na ordem do \
                      filme. Linha: grupo, total, imagens baixadas, imagens pendentes de acesso \
                      (faixas como `1-40,45`). Um filme restrito sai com 4."
    )]
    List {
        #[arg(value_name = "GRUPO", help = "O grupo de imagens (filme digital)")]
        film: Film,
        #[command(flatten)]
        folder: FolderArgs,
    },
    #[command(
        name = "baixar",
        about = "Baixa as imagens em resolução original, com manifesto e retomada",
        long_about = "Baixa as imagens em resolução original, com manifesto e retomada.\n\nCada \
                      imagem vai para PASTA como `<grupo>_<número>.jpg` e ganha uma linha em \
                      PASTA/manifesto.jsonl, com `sha256`, `bytes`, `largura` e `altura`. \
                      Repetir o comando retoma: pula a imagem cujo arquivo existe e confere \
                      com o hash do manifesto, e baixa as outras. A imagem que o armazenamento recusa a esta conta \
                      fica no manifesto como `pendente-de-acesso`, o download segue, e ela não \
                      é pedida de novo sem --tentar-pendentes. Qualquer outro 403 é bloqueio: \
                      o comando para com 4 e nenhuma outra imagem é pedida.\n\nOs pedidos ao FamilySearch saem um por vez, no \
                      ritmo da conta; só as transferências do armazenamento correm em paralelo. \
                      Depois de um bloqueio ou do teto da sessão, o que já veio fica guardado: \
                      repita o mesmo comando. Linha: grupo, baixadas agora, já havia, pendentes \
                      de acesso, pasta. O andamento sai na saída de erro."
    )]
    Download {
        #[arg(value_name = "GRUPO", help = "O grupo de imagens (filme digital)")]
        film: Film,
        #[command(flatten)]
        range: RangeArgs,
        #[command(flatten)]
        folder: FolderArgs,
        #[arg(
            long = "paralelo",
            default_value_t = 3,
            value_parser = clap::value_parser!(u8).range(1..=8),
            value_name = "N",
            help = "Quantas transferências ao mesmo tempo, até 8"
        )]
        workers: u8,
        #[arg(
            long = "tentar-pendentes",
            help = "Pede de novo as imagens que ficaram pendentes de acesso"
        )]
        retry_pending: bool,
    },
    #[command(
        name = "miniaturas",
        about = "Baixa as miniaturas do filme, para a triagem",
        long_about = "Baixa as miniaturas do filme, para a triagem.\n\nCada miniatura (200 px) \
                      vai para PASTA/miniaturas como `<grupo>_<número>.jpg`; a que já existe \
                      não é pedida de novo. Junte-as com `garimpo imagens folha`. A triagem \
                      localiza; nunca prova ausência. Miniatura em retrato costuma ser ficha \
                      ou capa. Linha: grupo, baixadas agora, já havia, pendentes de acesso, \
                      pasta."
    )]
    Thumbnails {
        #[arg(value_name = "GRUPO", help = "O grupo de imagens (filme digital)")]
        film: Film,
        #[command(flatten)]
        range: RangeArgs,
        #[command(flatten)]
        folder: FolderArgs,
    },
    #[command(
        name = "recortar",
        about = "Recorta a imagem em pedaços que o modelo lê sem reduzir, com ampliação \
                 opcional",
        long_about = "Recorta a imagem em pedaços que o modelo lê sem reduzir, com ampliação \
                      opcional.\n\nO modelo lê até 2576 px no lado maior. Sem --regiao, a \
                      imagem inteira é dividida no menor número de pedaços que cabem nesse \
                      limite, com uma pequena sobreposição. Com --regiao, só aquele trecho. \
                      Com --ampliar, cada pedaço sai ampliado e ainda dentro do limite. O \
                      original não muda: cada pedaço é um arquivo novo ao lado dele, \
                      `<nome>.r<x>-<y>-<largura>x<altura>[.a<fator>].jpg`. Linha: arquivo, x, y, \
                      largura e altura do trecho no original."
    )]
    Crop {
        #[arg(value_name = "ARQUIVO", help = "A imagem a recortar")]
        file: PathBuf,
        #[arg(
            long = "regiao",
            value_name = "X,Y,LARGURA,ALTURA",
            help = "O trecho a recortar, em pixels do original, a partir do canto de cima à \
                    esquerda"
        )]
        region: Option<Region>,
        #[arg(
            long = "ampliar",
            default_value_t = 1,
            value_parser = clap::value_parser!(u32).range(1..=4),
            value_name = "FATOR",
            help = "Amplia cada pedaço: 2 ou 3 para uma linha ou uma palavra"
        )]
        enlarge: u32,
    },
    #[command(
        name = "contraste",
        about = "Realça o contraste da imagem inteira, em tons de cinza",
        long_about = "Realça o contraste da imagem inteira, em tons de cinza.\n\nÉ uma operação \
                      global: os mesmos níveis para todos os pixels, sem tocar em nenhum trecho \
                      em particular. O original não muda: o resultado é \
                      `<nome>.contraste.jpg`, ao lado dele. Linha: arquivo."
    )]
    Contrast {
        #[arg(value_name = "ARQUIVO", help = "A imagem a realçar")]
        file: PathBuf,
    },
    #[command(
        name = "folha",
        about = "Monta uma folha de contato com várias imagens, para a triagem",
        long_about = "Monta uma folha de contato com várias imagens, para a triagem.\n\nAs \
                      imagens entram reduzidas, na ordem dada, da esquerda para a direita e de \
                      cima para baixo, e nada é escrito sobre elas. Linha, uma por imagem: \
                      linha e coluna na folha, arquivo. A folha localiza candidatos; a leitura \
                      é no original."
    )]
    Sheet {
        #[arg(value_name = "ARQUIVO", required = true, help = "As imagens, na ordem")]
        files: Vec<PathBuf>,
        #[arg(long = "saida", value_name = "ARQUIVO", help = "A folha, em JPEG")]
        output: PathBuf,
        #[arg(
            long = "colunas",
            default_value_t = 6,
            value_parser = clap::value_parser!(u32).range(1..=9),
            value_name = "N",
            help = "Quantas imagens por linha da folha"
        )]
        columns: u32,
    },
}

/// Where the images of a research go.
#[derive(Debug, Args)]
pub struct FolderArgs {
    #[arg(
        long = "pasta",
        default_value = "imagens",
        value_name = "PASTA",
        help = "A pasta das imagens, com o manifesto.jsonl dentro"
    )]
    pub folder: PathBuf,
}

/// Which images of a film or of a document a command covers.
#[derive(Debug, Args)]
pub struct RangeArgs {
    #[arg(
        long = "de",
        default_value_t = 1,
        value_parser = clap::value_parser!(u32).range(1..),
        value_name = "N",
        help = "A primeira imagem, contando do 1"
    )]
    pub from: u32,
    #[arg(
        long = "ate",
        value_name = "N",
        help = "A última imagem. Padrão: a última que existe"
    )]
    pub to: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("grupo inválido: use o número do grupo de imagens, de até 9 algarismos")]
pub struct InvalidFilm;

/// The number of an image group (a digital film), with the nine digits and
/// the leading zeros the image storage requires.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Film(String);

impl FromStr for Film {
    type Err = InvalidFilm;

    fn from_str(text: &str) -> Result<Self, InvalidFilm> {
        let valid = (1..=9).contains(&text.len()) && text.chars().all(|c| c.is_ascii_digit());
        if valid {
            Ok(Self(format!("{text:0>9}")))
        } else {
            Err(InvalidFilm)
        }
    }
}

impl fmt::Display for Film {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("região inválida: use X,Y,LARGURA,ALTURA em pixels, com largura e altura acima de zero")]
pub struct InvalidRegion;

/// A rectangle of an image, in pixels from its top left corner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Region {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

impl Region {
    /// The left, the top, the width and the height.
    #[must_use = "the parts are only useful to crop with"]
    pub fn parts(self) -> (u32, u32, u32, u32) {
        (self.x, self.y, self.width, self.height)
    }
}

impl FromStr for Region {
    type Err = InvalidRegion;

    fn from_str(text: &str) -> Result<Self, InvalidRegion> {
        let numbers: Vec<u32> = text
            .split(',')
            .map(|number| number.trim().parse())
            .collect::<Result<_, _>>()
            .map_err(|_| InvalidRegion)?;
        match numbers[..] {
            [x, y, width, height] if width > 0 && height > 0 => Ok(Self {
                x,
                y,
                width,
                height,
            }),
            _ => Err(InvalidRegion),
        }
    }
}

#[derive(Debug, Subcommand)]
pub enum DigitarqCommand {
    #[command(
        name = "buscar",
        about = "Busca nas descrições do digitarq",
        long_about = "Busca nas descrições do digitarq.\n\nA busca indexa o texto da descrição: \
                      nas habilitações, os pais e os avós citados. Sem --frase, todas as \
                      palavras, em qualquer ordem; aceita também uma cota (código de \
                      referência). Linha: id do documento, cota, datas, título, número de \
                      imagens. O id serve em `documento`, `imagens` e `baixar`."
    )]
    Search {
        #[arg(
            value_name = "TEXTO",
            help = "As palavras, a frase ou a cota a procurar"
        )]
        text: String,
        #[arg(long = "frase", help = "Procura o texto como uma frase exata")]
        phrase: bool,
        #[command(flatten)]
        page: PageArgs,
        #[command(flatten)]
        coverage: CoverageArgs,
    },
    #[command(
        name = "documento",
        about = "A descrição completa de um documento",
        long_about = "A descrição completa de um documento.\n\nLinha, uma por campo preenchido: \
                      campo, valor. `ScopeContent` é o âmbito e conteúdo: nas habilitações, \
                      naturalidade, pais e avós. É a descrição do arquivo, não o documento: \
                      etiqueta [Cat]."
    )]
    Document {
        #[arg(value_name = "ID", help = "O id do documento no digitarq")]
        id: DocumentId,
        #[command(flatten)]
        coverage: CoverageArgs,
    },
    #[command(
        name = "imagens",
        about = "As imagens de um documento, em ordem",
        long_about = "As imagens de um documento, em ordem.\n\nLinha: número, id do arquivo, \
                      nome, representação. Um documento pode ter duas representações (duas \
                      digitalizações): escolha uma em `baixar --representacao`. Há documentos \
                      sem imagem."
    )]
    Files {
        #[arg(value_name = "ID", help = "O id do documento no digitarq")]
        id: DocumentId,
        #[command(flatten)]
        coverage: CoverageArgs,
    },
    #[command(
        name = "baixar",
        about = "Baixa as imagens de um documento, com manifesto e retomada",
        long_about = "Baixa as imagens de um documento, com manifesto e retomada.\n\nCada \
                      imagem vai para PASTA com o nome que o digitarq lhe dá e ganha uma linha \
                      em PASTA/manifesto.jsonl, como em `garimpo imagens baixar`. Repetir o \
                      comando retoma. As imagens podem vir em TIFF: `garimpo imagens recortar` \
                      gera os pedaços em JPEG. Linha: documento, baixadas agora, já havia, \
                      pendentes de acesso, pasta."
    )]
    Download {
        #[arg(value_name = "ID", help = "O id do documento no digitarq")]
        id: DocumentId,
        #[arg(
            long = "representacao",
            value_name = "ID",
            help = "Baixa só as imagens dessa representação"
        )]
        representation: Option<String>,
        #[command(flatten)]
        range: RangeArgs,
        #[command(flatten)]
        folder: FolderArgs,
        #[command(flatten)]
        coverage: CoverageArgs,
    },
    #[command(
        name = "oai",
        about = "Colhe as descrições de um arquivo pelo OAI-PMH, para um arquivo local",
        long_about = "Colhe as descrições de um arquivo pelo OAI-PMH, para um arquivo local.\n\n\
                      CONJUNTO é o código do arquivo: `ANTT` (Torre do Tombo), `ADPRT` (Porto), \
                      `ADFAR` (Faro) e os outros distritais. --desde e --ate filtram pela data \
                      em que a descrição mudou, não pela data do documento. Cada rodada colhe \
                      até --paginas páginas de 100 descrições e grava em --saida; repetir o \
                      comando continua de onde parou. Linha: descrições no arquivo, total, \
                      arquivo. Depois procure os nomes com `garimpo busca nomes <arquivo> \
                      <nome>…`, que não chama rede nenhuma."
    )]
    Oai {
        #[arg(value_name = "CONJUNTO", help = "O código do arquivo, como ANTT")]
        set: RouteId,
        #[arg(long = "saida", value_name = "ARQUIVO", help = "O arquivo da colheita")]
        output: PathBuf,
        #[arg(
            long = "desde",
            value_name = "AAAA-MM-DD",
            help = "Só o que mudou a partir desta data"
        )]
        from: Option<chrono::NaiveDate>,
        #[arg(
            long = "ate",
            value_name = "AAAA-MM-DD",
            help = "Só o que mudou até esta data"
        )]
        until: Option<chrono::NaiveDate>,
        #[arg(
            long = "paginas",
            default_value_t = 10,
            value_parser = clap::value_parser!(u32).range(1..),
            value_name = "N",
            help = "Quantas páginas colher nesta rodada. Cada uma leva cerca de 10 s"
        )]
        pages: u32,
        #[command(flatten)]
        coverage: CoverageArgs,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("id inválido: use o id do documento, só letras e números, como sai de `digitarq buscar`")]
pub struct InvalidDocumentId;

/// The identifier of a digitarq document: only what can go into a URL as it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentId(String);

impl FromStr for DocumentId {
    type Err = InvalidDocumentId;

    fn from_str(text: &str) -> Result<Self, InvalidDocumentId> {
        if !text.is_empty() && text.chars().all(|c| c.is_ascii_alphanumeric()) {
            Ok(Self(text.to_owned()))
        } else {
            Err(InvalidDocumentId)
        }
    }
}

impl fmt::Display for DocumentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Args)]
pub struct DiagramArgs {
    #[arg(
        value_name = "DESCRICAO",
        help = "O arquivo JSON com a descrição da linhagem, ou `-`"
    )]
    pub description: PathBuf,
    #[arg(
        long = "formato",
        value_enum,
        default_value_t = DiagramFormat::Mermaid,
        value_name = "FORMATO",
        help = "O formato do desenho",
        long_help = "O formato do desenho.\n\n`mermaid` vai num bloco de código `mermaid`. \
                     `texto` é para colar onde desenho não entra, como uma mensagem. `svg` é a \
                     imagem com tema, desenhada pelo Pele a partir do Mermaid: precisa do `bun` \
                     e de `bun install --frozen-lockfile` na pasta `svg/` do kit."
    )]
    pub format: DiagramFormat,
    #[arg(
        long = "tema",
        value_enum,
        default_value_t = Theme::Plain,
        value_name = "TEMA",
        help = "O estilo do SVG"
    )]
    pub theme: Theme,
    #[arg(
        long = "saida",
        value_name = "ARQUIVO",
        help = "Grava o desenho no arquivo. A linha passa a ser: arquivo, número de furos"
    )]
    pub output: Option<PathBuf>,
    #[arg(
        long = "renderizador",
        env = "GARIMPO_RENDERIZADOR",
        default_value = concat!(env!("CARGO_MANIFEST_DIR"), "/svg/render.ts"),
        hide_default_value = true,
        value_name = "ARQUIVO",
        help = "O programa do kit que desenha o SVG. Padrão: `svg/render.ts` da cópia do kit \
                de onde o garimpo foi compilado"
    )]
    pub renderer: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum DiagramFormat {
    #[value(name = "mermaid")]
    Mermaid,
    #[value(name = "texto")]
    Text,
    #[value(name = "svg")]
    Svg,
}

/// The look of the SVG drawing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Theme {
    #[value(name = "simples")]
    Plain,
    #[value(name = "pergaminho")]
    Parchment,
    #[value(name = "planta")]
    Blueprint,
    #[value(name = "familysearch")]
    FamilySearch,
}
