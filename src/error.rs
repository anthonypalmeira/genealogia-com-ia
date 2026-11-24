//! The errors an agent has to tell apart, each with its own exit code and the
//! next step to take. Messages are in Portuguese, like the rest of the CLI.

use crate::familysearch::{PersonId, SourceId};

/// A failed `garimpo` command.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An argument, identifier or input file that cannot be used.
    #[error("entrada inválida: {0}")]
    Input(String),
    /// No session was delivered, or its holder has already ended.
    #[error("login necessário: nenhuma sessão do FamilySearch foi entregue")]
    SessionMissing,
    /// FamilySearch rejected the held session (401).
    #[error("login necessário: a sessão do FamilySearch expirou")]
    SessionExpired,
    /// FamilySearch blocked the request: 403, or 429 even after the pauses.
    #[error("bloqueio: o FamilySearch respondeu {status} em {path}")]
    Blocked { status: u16, path: String },
    /// FamilySearch asked for a pause longer than the command accepts to wait.
    #[error("bloqueio: o FamilySearch pediu uma pausa e ainda faltam {seconds} s")]
    Paused { seconds: u64 },
    /// The session hit its request cap.
    #[error("teto da sessão atingido: {cap} requisições")]
    SessionCap { cap: u32 },
    /// A write without the round the user authorized.
    #[error("escrita recusada: falta a rodada autorizada pelo usuário")]
    WriteNotAuthorized,
    /// The source was created, but attaching it to the person failed.
    #[error("a fonte {source_id} foi criada, mas não anexada a {person}: {cause:#}")]
    SourceNotAttached {
        source_id: SourceId,
        person: PersonId,
        cause: Box<Error>,
    },
    /// Anything else: network, disk, unexpected response.
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl Error {
    /// The process exit code for this error.
    #[must_use = "the code is only useful to end the process with it"]
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::Other(_) => 1,
            Self::Input(_) => 2,
            Self::SessionMissing | Self::SessionExpired => 3,
            Self::Blocked { .. } | Self::Paused { .. } => 4,
            Self::SessionCap { .. } => 5,
            Self::WriteNotAuthorized => 6,
            Self::SourceNotAttached { cause, .. } => cause.exit_code(),
        }
    }

    /// What the agent should do next.
    #[must_use = "the next step is only useful when shown to the agent"]
    pub fn next_step(&self) -> String {
        match self {
            Self::Other(_) => "leia a causa acima; se for passageira, rode de novo".into(),
            Self::Input(_) => "corrija a entrada (veja `garimpo --help`) e rode de novo".into(),
            Self::SessionMissing | Self::SessionExpired => {
                "peça ao usuário para entrar no FamilySearch, entregue a sessão nova com \
                 `garimpo sessao entregar` e rode de novo"
                    .into()
            }
            Self::Blocked { .. } => {
                "pare de chamar o FamilySearch por agora e avise o usuário; bloqueio não \
                 significa que o dado não existe"
                    .into()
            }
            Self::Paused { seconds } => format!(
                "espere {seconds} s e rode de novo o mesmo comando; antes disso o garimpo \
                 recusa sem chamar o FamilySearch; bloqueio não significa que o dado não existe"
            ),
            Self::SessionCap { .. } => {
                "pare; só o usuário decide abrir outra sessão (`garimpo sessao entregar`) \
                 ou subir GARIMPO_TETO_SESSAO"
                    .into()
            }
            Self::WriteNotAuthorized => {
                "mostre o plano de gravação ao usuário e, com o sim dele, repita com \
                 `--rodada <identificação da rodada>`"
                    .into()
            }
            Self::SourceNotAttached {
                source_id, person, ..
            } => format!(
                "não repita `criar-fonte`, que criaria outra fonte; resolvida a causa, anexe a \
                 que já existe com `garimpo fs anexar-fonte {person} --fonte {source_id}` e os \
                 mesmos --motivo e --rodada"
            ),
        }
    }
}
