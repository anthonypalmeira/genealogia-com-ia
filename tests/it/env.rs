//! An isolated environment to run the `garimpo` binary: its own state
//! directory, its own working directory, and the base URLs pointed at the fake
//! server.

use std::{
    env, fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::atomic::{AtomicU32, Ordering},
};

use serde_json::Value;

use crate::server::FakeServer;

/// Synthetic: no real token ever enters the tests.
pub(crate) const TOKEN: &str = "token-sintetico-de-teste";

static NEXT: AtomicU32 = AtomicU32::new(0);

pub(crate) struct Output {
    pub(crate) code: i32,
    pub(crate) stdout: String,
    pub(crate) stderr: String,
}

pub(crate) struct TestEnv {
    state_dir: PathBuf,
    work_dir: PathBuf,
    api: String,
    extra: Vec<(String, String)>,
}

impl TestEnv {
    pub(crate) fn new(server: &FakeServer) -> Self {
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let unique = format!("garimpo-{}-{n}", std::process::id());
        // The state directory holds the session socket, and a Unix socket path
        // cannot exceed ~100 bytes. So it lives in the system temp directory,
        // which is short; everything else lives in Cargo's test directory.
        let state_dir = env::temp_dir().join(&unique);
        let work_dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(&unique);
        for dir in [&state_dir, &work_dir] {
            // Left over from an earlier run with the same process id; usually
            // the directory does not even exist.
            let _ = fs::remove_dir_all(dir);
            fs::create_dir_all(dir).unwrap();
        }
        Self {
            state_dir,
            work_dir,
            api: server.url().to_owned(),
            extra: Vec::new(),
        }
    }

    /// An environment with the synthetic session already delivered.
    pub(crate) fn logged_in(server: &FakeServer) -> Self {
        let env = Self::new(server);
        env.deliver_session();
        env
    }

    /// Adds an environment variable to every following command.
    pub(crate) fn with_var(mut self, name: &str, value: &str) -> Self {
        self.extra.push((name.into(), value.into()));
        self
    }

    pub(crate) fn state_dir(&self) -> &Path {
        &self.state_dir
    }

    pub(crate) fn work_dir(&self) -> &Path {
        &self.work_dir
    }

    /// The coverage log the searches of this environment write, one record
    /// per line.
    pub(crate) fn coverage(&self) -> Vec<Value> {
        fs::read_to_string(self.work_dir.join("cobertura.jsonl"))
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    }

    pub(crate) fn command(&self, args: &[&str]) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_garimpo"));
        command
            .args(args)
            .current_dir(&self.work_dir)
            .env_clear()
            .env("GARIMPO_DIR", &self.state_dir)
            .env("GARIMPO_FS_API", &self.api)
            .env("GARIMPO_FS_IDENT", &self.api)
            .env("GARIMPO_FS_SITE", &self.api)
            .env("GARIMPO_FS_IMAGENS", &self.api)
            .env("GARIMPO_DIGITARQ", &self.api)
            // No wait between images, unless a test asks for one.
            .env("GARIMPO_INTERVALO_IMAGENS_MS", "0")
            // So coverage also measures the binary, when it is on.
            .envs(env::var_os("LLVM_PROFILE_FILE").map(|profile| ("LLVM_PROFILE_FILE", profile)))
            .envs(self.extra.iter().map(|(name, value)| (name, value)))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        command
    }

    pub(crate) fn run(&self, args: &[&str]) -> Output {
        output(self.command(args).spawn().unwrap())
    }

    pub(crate) fn run_with_stdin(&self, args: &[&str], stdin: &str) -> Output {
        let mut child = self.command(args).stdin(Stdio::piped()).spawn().unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(stdin.as_bytes())
            .unwrap();
        output(child)
    }

    /// Delivers the synthetic session, as the agent does once per login.
    pub(crate) fn deliver_session(&self) {
        let delivered = self.run_with_stdin(&["sessao", "entregar"], TOKEN);
        assert_eq!(delivered.code, 0, "{}", delivered.stderr);
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        // Ends the processes that hold the session and the bridge, if there
        // are any. This runs while a failed test unwinds too, so it must not
        // panic; the result does not matter because the test is already over.
        let _ = self.command(&["sessao", "encerrar"]).output();
        let _ = self.command(&["ponte", "fechar"]).output();
        // Cleanup: a leftover temporary directory does not invalidate the test.
        let _ = fs::remove_dir_all(&self.state_dir);
        let _ = fs::remove_dir_all(&self.work_dir);
    }
}

pub(crate) fn output(child: Child) -> Output {
    let output = child.wait_with_output().unwrap();
    Output {
        code: output.status.code().unwrap(),
        stdout: String::from_utf8(output.stdout).unwrap(),
        stderr: String::from_utf8(output.stderr).unwrap(),
    }
}
