use crate::cmd::{Execute, Config};
use crate::config_man::set_analytics;
use anyhow::{Context, Result};

impl Execute for Config {
    async fn execute(&self) -> Result<()> {
        if self.analytics.is_some() {
            let analytics = self.analytics.context("Make sure to set analytics to true or false")?;
            set_analytics(analytics).context("Failed to set analytics")?;
            println!("Analytics set to: {}", analytics);
        }
        Ok(())
    }
}