pub mod app;
mod commands;
pub mod domain;
pub mod error;
pub mod github;
pub mod monitor;
pub mod storage;

pub fn run() {
    app::run()
}
