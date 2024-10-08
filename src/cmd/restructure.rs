use crate::cmd::{Execute, Restructure};
use anyhow::{Result, Context};
use parsv::get_submodules;
use crate::toml;
// use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

impl Execute for Restructure {
    async fn execute(&self) -> Result<()> {
        let top_module_path: &Path = Path::new(&self.top_module_path);
        if !Path::new(top_module_path).exists() {
            return Err(anyhow::anyhow!("Top module file does not exist. Ensure the path is correct."));
        }

        // let pb = ProgressBar::new_spinner();
        // pb.set_style(ProgressStyle::default_spinner());
        // pb.enable_steady_tick(std::time::Duration::from_millis(500));
        // pb.set_message("Restructuring top module");

        let mut path_set: HashSet<PathBuf> = HashSet::new();
        path_set.insert(top_module_path.to_path_buf());

        let ext = top_module_path.extension().context("Failed to get extension of top module file. Ensure the path is correct.")?.to_str().unwrap();
        let top_module_name = top_module_path.file_name().unwrap().to_str().unwrap().split('.').next().unwrap();
        let top_module_contents = std::fs::read_to_string(top_module_path).context("Failed to read top module file. Ensure the path is correct.")?;
        let submodules = get_submodules(&top_module_contents).context("Failed to get submodules")?;

        // pb.set_message("Restructuring submodules");
        println!("Restructuring modules");
        println!("Top module name: {}", top_module_name);
        for submodule_name in submodules {
            println!("Submodule name: {}", submodule_name);
            let submodule_path = top_module_path.with_file_name(format!("{submodule_name}.{ext}"));
            if !submodule_path.exists() {
                println!("Submodule file '{}' not found.", submodule_path.display());
                print!("Please enter the correct path for the '{}' submodule: ", submodule_name);
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

        let response = loop {
            print!("Would you like to move or copy the files? (move/copy): ");
            std::io::Write::flush(&mut std::io::stdout())?;
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            let input = input.trim().to_lowercase();
            if input == "move" || input == "copy" {
                break input;
            }
            println!("Invalid input. Please enter 'move' or 'copy'.");
        };

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

        let version = loop {
            print!("Please enter a version number for this module: ");
            std::io::Write::flush(&mut std::io::stdout())?;
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            let input = input.trim();
            if !input.is_empty() {
                break input.to_string();
            }
            println!("Invalid input. Please enter a non-empty version number.");
        };
        println!("Module version set to: {}", version);

        let origin = loop {
            print!("Please enter the origin (e.g., GitHub URL) for this module: ");
            std::io::Write::flush(&mut std::io::stdout())?;
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            let input = input.trim();
            if !input.is_empty() {
                break input.to_string();
            }
            println!("Invalid input. Please enter a non-empty origin.");
        };
        println!("Module origin set to: {}", origin);

        toml::add_dependency(&origin).context("Failed to add dependency to toml file.")?;
        toml::add_top_module(&origin,&format!("vpm_modules/{}/rtl/{}.{}", top_module_name, top_module_name, ext), &version).context("Failed to add top module to toml file.")?;

        Ok(())
    }
}