use anyhow::Result;

use crate::cmd::{Execute, Tune};
use clust::Client;
use clust::messages::{Message, MessagesRequestBody, ClaudeModel, MaxTokens};
use rand::Rng;

impl Execute for Tune {
    async fn execute(&self) -> Result<()> {
        println!("Tuning module: '{}' to module: '{}'", &self.source_module, &self.target_module);
        Ok(())
    }
}