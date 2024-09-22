use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use anyhow::{Result, Context};

use crate::cmd::{Execute, Update};
use crate::cmd::include::{get_head_commit_hash, include_repo_from_url, name_from_url};
use crate::toml::{get_repo_links, add_top_module, remove_top_module};
use imara_diff::intern::InternedInput;
use imara_diff::{diff, Algorithm, UnifiedDiffBuilder};

impl Execute for Update {
    async fn execute(&self) -> Result<()> {
        let module_path = &self.module_path;
        println!("Updating module '{}'", module_path);
        update_module(module_path, self.commit.as_deref(), true).context("Failed to update module. Ensure the path is correct and the file exists.")?;
        Ok(())
    }
}

fn update_module(module_path: &str, commit: Option<&str>, is_top_module: bool) -> Result<()> {
    let repo_links = get_repo_links(module_path);
    if repo_links.is_empty() {
        return Err(anyhow::anyhow!("No repositories found for module '{}'", module_path));
    }

    let chosen_repo = if repo_links.len() == 1 {
        repo_links.into_iter().next().unwrap()
    } else {
        println!("Multiple repositories found for module '{}'. Please choose one:", module_path);
        for (index, link) in repo_links.iter().enumerate() {
            println!("{}. {}", index + 1, link);
        }
        let mut choice = String::new();
        std::io::stdin().read_line(&mut choice)?;
        let index: usize = choice.trim().parse()?;
        repo_links.into_iter().nth(index - 1)
            .ok_or_else(|| anyhow::anyhow!("Invalid choice"))?
    };

    let head_commit_hash = get_head_commit_hash(&chosen_repo).unwrap();
    let commit_hash = commit.unwrap_or(&head_commit_hash);

    println!("Preparing to update module '{}' to commit '{}'", module_path, commit_hash);
    let old_contents = std::fs::read_to_string(module_path).context(format!("Failed to read module '{}'", module_path))?;
    
    let (_chosen_file, new_contents) = find_and_choose_module_file(&chosen_repo, module_path, commit_hash)?;

    let temp_path = format!("{}.temp", module_path);
    std::fs::write(&temp_path, &new_contents)?;

    let ext = Path::new(module_path).extension().unwrap_or_default().to_str().unwrap_or(".v");

    // Display the diff and ask for confirmation
    display_diff(&old_contents, &new_contents);

    print!("Do you want to apply these changes? (y/n): ");
    io::stdout().flush().unwrap();
    let mut choice = String::new();
    std::io::stdin().read_line(&mut choice)?;

    if choice.trim().to_lowercase() == "y" {
        // Apply the changes
        std::fs::rename(&temp_path, module_path)?;
        println!("Module '{}' updated to commit '{}'", module_path, commit_hash);
    } else {
        // Remove the temporary file
        std::fs::remove_file(&temp_path)?;
        println!("Update cancelled. No changes were made.");
    }

    // Ask if the user wants to update submodules
    print!("Would you like to update submodules as well? (y/n): ");
    io::stdout().flush().unwrap();
    let mut submodule_choice = String::new();
    std::io::stdin().read_line(&mut submodule_choice)?;

    if submodule_choice.trim().to_lowercase() == "y" {
        // Get submodules from the updated module
        let submodules = parsv::get_submodules(&new_contents).context("Failed to get submodules")?;
        
        for submodule in submodules {
            let submodule_path = Path::new(module_path).with_file_name(format!("{}.{}", submodule, ext));
            if submodule_path.exists() {
                println!("Preparing to update submodule: {}", submodule);
                let old_contents = fs::read_to_string(&submodule_path)?;
                
                // Use find_and_choose_module_file for submodules
                let (_, new_contents) = find_and_choose_module_file(&chosen_repo, &submodule_path.to_str().unwrap(), commit_hash)?;
                
                // Create a temporary file for the new contents
                let temp_path = format!("{}.temp", submodule_path.display());
                fs::write(&temp_path, &new_contents)?;
                
                println!("Changes for submodule {}:", submodule);
                display_diff(&old_contents, &new_contents);

                print!("Do you want to apply these changes? (y/n): ");
                io::stdout().flush().unwrap();
                let mut choice = String::new();
                std::io::stdin().read_line(&mut choice)?;

                if choice.trim().to_lowercase() == "y" {
                    // Apply the changes for the submodule
                    fs::rename(&temp_path, &submodule_path).context(format!("Failed to update submodule '{}'", submodule))?;
                    println!("Submodule '{}' updated to commit '{}'", submodule, commit_hash);
                } else {
                    println!("Skipping submodule '{}'", submodule);
                    fs::remove_file(&temp_path)?;
                }
                
            } else {
                println!("Submodule file not found: {}. Skipping...", submodule_path.display());
            }
        }
    }

    if is_top_module {
        remove_top_module(&chosen_repo, &temp_path)?;
        add_top_module(&chosen_repo, &temp_path, commit_hash)?;
    }

    Ok(())
}

fn find_and_choose_module_file(chosen_repo: &str, module_path: &str, commit_hash: &str) -> Result<(PathBuf, String)> {
    include_repo_from_url(chosen_repo, "/tmp/", Some(commit_hash), false)?;
    
    let module_name = Path::new(module_path).file_stem().unwrap().to_str().unwrap();
    let tmp_path = PathBuf::from("/tmp").join(name_from_url(chosen_repo));
    
    let matching_files = find_matching_files(&tmp_path, module_name);
    
    let chosen_file = if matching_files.is_empty() {
        return Err(anyhow::anyhow!("No matching files found for module '{}'", module_name));
    } else if matching_files.len() == 1 {
        matching_files[0].clone()
    } else {
        println!("Multiple matches found for module '{}'. Please choose one:", module_name);
        for (index, file) in matching_files.iter().enumerate() {
            println!("{}. {}", index + 1, file.strip_prefix(&tmp_path).unwrap().display());
        }
        let mut choice = String::new();
        std::io::stdin().read_line(&mut choice)?;
        let index: usize = choice.trim().parse()?;
        matching_files.get(index - 1)
            .ok_or_else(|| anyhow::anyhow!("Invalid choice"))?
            .clone()
    };

    let new_contents = std::fs::read_to_string(&chosen_file)?;
    Ok((chosen_file, new_contents))
}

fn find_matching_files(tmp_path: &PathBuf, module_name: &str) -> Vec<PathBuf> {
    walkdir::WalkDir::new(tmp_path)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| {
            entry.file_name().to_str() == Some(&format!("{}.sv", module_name)) || 
            entry.file_name().to_str() == Some(&format!("{}.v", module_name))
        })
        .map(|entry| entry.path().to_path_buf())
        .collect()
}

fn display_diff(old_contents: &str, new_contents: &str) {
    let input = InternedInput::new(old_contents, new_contents);
    let mut diff_output = diff(
        Algorithm::Histogram,
        &input,
        UnifiedDiffBuilder::new(&input)
    );
    if diff_output.is_empty() {
        diff_output = "No differences found.".to_string();
    }

    println!("Diff:\n{}", diff_output);
}