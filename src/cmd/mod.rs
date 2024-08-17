mod cmd;
mod docs;
mod install;
mod uninstall;
mod dotf;
mod list;
mod chat;
mod tune;

use anyhow::Result;

pub use crate::cmd::cmd::*;

pub trait Execute {
    async fn execute(&self) -> Result<()>;
}

impl Execute for Cmd {
    async fn execute(&self) -> Result<()> {
        match self {
            Cmd::Install(cmd) => cmd.execute().await,
            Cmd::Uninstall(cmd) => cmd.execute().await,
            Cmd::Chat(cmd) => cmd.execute().await,
            Cmd::Docs(cmd) => cmd.execute().await,
            Cmd::Dotf(cmd) => cmd.execute().await,
            Cmd::List(cmd) => cmd.execute().await,
            Cmd::Tune(cmd) => cmd.execute().await,
        }
    }
}