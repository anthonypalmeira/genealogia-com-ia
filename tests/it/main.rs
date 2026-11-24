//! End-to-end tests: they run the compiled `garimpo` binary against a fake
//! FamilySearch and check only what is visible from outside (output, files,
//! logs and exit code). The CLI surface under test is in Portuguese.

#![expect(
    clippy::unwrap_used,
    reason = "in test helpers, an `unwrap` that fails is the test failing"
)]

mod env;
mod server;

mod pacing;
mod read;
mod session;
mod traversal;
mod write;
