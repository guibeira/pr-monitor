pub mod app;
mod commands;
mod credentials;
mod diagnostics;
pub mod domain;
pub mod error;
pub mod github;
pub mod monitor;
pub mod storage;

pub fn run() {
    app::run()
}
