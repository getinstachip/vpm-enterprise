use crate::cmd::{Execute, Restructure};
use anyhow::{Result, Context};
use parsv::get_submodules;
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

impl Execute for Restructure {
    async fn execute(&self) -> Result<()> {
        let top_module_path: &Path = Path::new(&self.top_module_path);
        if !Path::new(top_module_path).exists() {
            return Err(anyhow::anyhow!("Top module file does not exist. Ensure the path is correct."));
        }

        let pb = ProgressBar::new_spinner();
        pb.set_style(ProgressStyle::default_spinner());
        pb.enable_steady_tick(std::time::Duration::from_millis(500));
        pb.set_message("Restructuring top module");

        let mut path_set: HashSet<PathBuf> = HashSet::new();
        path_set.insert(top_module_path.to_path_buf());

        let ext = top_module_path.extension().context("Failed to get extension of top module file. Ensure the path is correct.")?.to_str().unwrap();
        let top_module_name = top_module_path.file_name().unwrap().to_str().unwrap().split('.').next().unwrap();
        let top_module_contents = std::fs::read_to_string(top_module_path).context("Failed to read top module file. Ensure the path is correct.")?;
        let submodules = get_submodules(&top_module_contents).context("Failed to get submodules")?;

        pb.set_message("Restructuring submodules");
        for submodule_name in submodules {
            let submodule_path = top_module_path.with_file_name(format!("{submodule_name}.{ext}"));
            if !submodule_path.exists() {
                println!("Submodule file '{}' not found.", submodule_path.display());
                println!("Please enter the correct path for the '{}' submodule:", submodule_name);
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;
                let input_path = input.trim();
                let new_submodule_path = PathBuf::from(input_path);
                
                if !new_submodule_path.exists() {
                    return Err(anyhow::anyhow!("Provided submodule path does not exist."));
                }
                
                path_set.insert(new_submodule_path);
                continue;
            }
            path_set.insert(submodule_path);
        }

        print!("Would you like to move or copy the files? (move/copy): ");
        let mut response = String::new();
        std::io::stdin().read_line(&mut response)?;
        let response = response.trim().to_lowercase();

        let new_dir_name = format!("vpm_modules/{}/rtl", top_module_name);
        let new_dir = Path::new(&new_dir_name);
        fs::create_dir_all(new_dir)?;
        for path in path_set {
            let new_path = new_dir.join(path.file_name().unwrap());
            let result = fs::write(&new_path, fs::read(&path)?);
            if let Err(e) = result {
                println!("Failed to write to new file {}: {}. Skipping...", new_path.display(), e);
                continue;
            }
            if response == "move" {
                fs::remove_file(&path)?;
            }
        }   

        Ok(())
    }
}