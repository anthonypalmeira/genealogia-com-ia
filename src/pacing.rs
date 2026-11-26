//! The account's pacing regulator, shared between processes.
//!
//! FamilySearch limits, per user, the processing time spent in a window, and
//! answers 429 with `Retry-After` when the limit is exceeded. The pacing state
//! lives in the state directory, and a lock next to it makes the processes of
//! one account talk to the server one at a time.

use std::{
    fs::{self, File, OpenOptions, TryLockError},
    io::ErrorKind,
    path::PathBuf,
    time::Duration,
};

use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::{cli::Config, error::Error};

#[derive(Debug, Default, Serialize, Deserialize)]
struct State {
    session_requests: u32,
    /// End of the pause the server asked for, in milliseconds since 1970.
    paused_until_ms: u64,
    /// The requests that still count in the window, oldest first.
    window: Vec<Spend>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct Spend {
    at_ms: u64,
    cost_ms: u64,
}

impl Spend {
    /// When this spend stops counting against the budget.
    fn leaves_window_ms(self, config: &Config) -> u64 {
        self.at_ms
            .saturating_add(config.window_s.saturating_mul(1000))
    }
}

/// A duration in whole milliseconds.
#[must_use = "the conversion has no effect of its own"]
pub(crate) fn millis(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

/// One process's turn to talk to FamilySearch. While it exists, no other
/// process of the same account makes a request.
#[derive(Debug)]
pub(crate) struct Turn {
    /// Holds the account lock until the turn ends.
    _lock: File,
    path: PathBuf,
    state: State,
}

impl Turn {
    /// Waits for the account lock and reads the pacing state.
    pub(crate) fn take(config: &Config) -> Result<Self, Error> {
        let dir = config.state_dir()?;
        let path = dir.join("pacing.json");
        let context = || format!("não consegui usar o ritmo em {}", dir.display());
        // The lock is a separate file because the state file is replaced whole
        // on every save.
        let lock = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .open(dir.join("pacing.lock"))
            .with_context(context)?;
        match lock.try_lock() {
            Ok(()) => {}
            Err(TryLockError::WouldBlock) => {
                eprintln!("garimpo: aguardando outro processo garimpo da mesma conta");
                lock.lock().with_context(context)?;
            }
            Err(TryLockError::Error(error)) => {
                return Err(anyhow::Error::new(error).context(context()).into());
            }
        }
        let state = match fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str(&text).with_context(context)?,
            Err(error) if error.kind() == ErrorKind::NotFound => State::default(),
            Err(error) => return Err(anyhow::Error::new(error).context(context()).into()),
        };
        Ok(Self {
            _lock: lock,
            path,
            state,
        })
    }

    /// Returns `Error::SessionCap` once the held session has made every
    /// request it was allowed.
    pub(crate) fn capped(&self, config: &Config) -> Result<(), Error> {
        if self.state.session_requests >= config.session_cap {
            return Err(Error::SessionCap {
                cap: config.session_cap,
            });
        }
        Ok(())
    }

    /// How long until the next request fits the pause and the window budget.
    #[must_use = "the request must wait this long"]
    pub(crate) fn wait(&self, config: &Config, now_ms: u64) -> Duration {
        let state = &self.state;
        let in_window = || {
            state
                .window
                .iter()
                .filter(|spend| spend.leaves_window_ms(config) > now_ms)
        };
        let mut spent_ms: u64 = in_window().map(|spend| spend.cost_ms).sum();
        let mut free_at_ms = now_ms;
        for spend in in_window() {
            if spent_ms < config.budget_ms {
                break;
            }
            spent_ms -= spend.cost_ms;
            free_at_ms = spend.leaves_window_ms(config);
        }
        let until_ms = free_at_ms.max(state.paused_until_ms);
        Duration::from_millis(until_ms.saturating_sub(now_ms))
    }

    /// Counts one request, with the cost the server reported and the pause it
    /// asked for, and saves the state. The turn goes on until it is dropped.
    pub(crate) fn record(
        &mut self,
        config: &Config,
        now_ms: u64,
        cost_ms: u64,
        pause: Option<Duration>,
    ) -> Result<(), Error> {
        self.state
            .window
            .retain(|spend| spend.leaves_window_ms(config) > now_ms);
        // The clock is read in whole milliseconds, rounded down. Counting this
        // request and its pause from the next one keeps a pause or a window
        // from ending a moment early.
        let now_ms = now_ms.saturating_add(1);
        self.state.window.push(Spend {
            at_ms: now_ms,
            cost_ms,
        });
        self.state.session_requests += 1;
        if let Some(pause) = pause {
            self.state.paused_until_ms = now_ms.saturating_add(millis(pause));
        }
        self.save()
    }

    /// Resets the request count: a new session has started. The pause and the
    /// window still hold, because the limit belongs to the account, not to the
    /// session.
    pub(crate) fn new_session(mut self) -> Result<(), Error> {
        self.state.session_requests = 0;
        self.save()
    }

    /// Replaces the state file in one step, so an interrupted process cannot
    /// leave half a state behind (which would erase the pause and the cap count).
    fn save(&self) -> Result<(), Error> {
        let text =
            serde_json::to_string(&self.state).context("não consegui gerar o estado do ritmo")?;
        let draft = self.path.with_extension("json.new");
        fs::write(&draft, text)
            .and_then(|()| fs::rename(&draft, &self.path))
            .with_context(|| format!("não consegui gravar {}", self.path.display()))?;
        Ok(())
    }
}
