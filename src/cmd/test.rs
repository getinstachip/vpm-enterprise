use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;
use parsv;

use crate::cmd::{Execute, Test};

impl Execute for Test {
    async fn execute(&self) -> Result<()> {
        let module_path = PathBuf::from(&self.module_path);
        let content = parsv::read_file(&module_path.to_str().unwrap()).context("Failed to read module content. Ensure the path is correct and the file exists.")?;
        let sims_path = PathBuf::from(&module_path.parent().unwrap().parent().unwrap().join("sims"));

        println!("Generating documentation for module: {}", module_path.to_str().unwrap());
        let pb = ProgressBar::new_spinner();
        pb.set_style(ProgressStyle::default_spinner().template("{spinner} {msg}").unwrap());

        pb.set_message("Drafting testbenches...");
        pb.enable_steady_tick(std::time::Duration::from_millis(100));
        parsv::generate_testbenches(&content, &sims_path.to_str().unwrap(), 1, 2, 2).await.context("Failed to generate testbenches. Try again, and if the error persists, report it to the developers.")?;

        pb.set_message("Running simulations...");
        pb.enable_steady_tick(std::time::Duration::from_millis(100));
        parsv::run_testbenches(&module_path.to_str().unwrap(), &sims_path.to_str().unwrap()).context("Failed to run testbenches. Try again, and if the error persists, report it to the developers.")?;

        pb.set_message("Analyzing waveforms...");
        pb.enable_steady_tick(std::time::Duration::from_millis(100));
        parsv::generate_waveform_images(&sims_path.to_str().unwrap()).context("Failed to generate waveform images. Try again, and if the error persists, report it to the developers.")?;

        pb.set_message("Finalizing simulations...");
        pb.enable_steady_tick(std::time::Duration::from_millis(100));

        pb.finish_with_message(format!("Simulations complete. Files saved to: {}", sims_path.to_str().unwrap()));
        Ok(())
    }
}
