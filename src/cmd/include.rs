use crate::cmd::{Execute, Include};
use crate::cmd::docs::{generate_docs, generate_docs_offline};
use crate::toml::{add_dependency, add_top_module};

use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent},
    execute,
    style::{Print, ResetColor, SetBackgroundColor, SetForegroundColor, Color},
    terminal::{
        disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};
use std::collections::HashSet;
use std::env::current_dir;
use std::path::{Path, PathBuf};
use std::{fs, process::Command};
use anyhow::{Context, Result};
use parsv::{get_submodules, generate_headers};
use walkdir::{DirEntry, WalkDir};

use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use std::io::{self, Write};
use indicatif::{ProgressBar, ProgressStyle};

impl Execute for Include {
    async fn execute(&self) -> Result<()> {
        fs::create_dir_all("./vpm_modules")?;
        println!("Including from: '{}'", self.url);
        let repo_name = name_from_url(&self.url);
        let mut is_head = false;
        let tmp_path = PathBuf::from("/tmp").join(repo_name);
        let commit = if self.commit.is_none() {
            is_head = true;
            Some(get_head_commit_hash(&self.url).context("Failed to get HEAD commit hash. Please check your internet connection and ensure the provided URL is correct.")?)
        } else {
            self.commit.clone()
        };

        let included_modules: HashSet<String> = if self.repo {
            include_entire_repo(&self.url, &tmp_path, self.riscv, commit.as_deref(), is_head).context("Failed to include entire repository")?
        } else {
            include_single_module(&self.url, self.riscv, commit.as_deref(), is_head).context("Failed to include single module")?
        };

        if self.with_docs {
            for module in included_modules {
                let module_content = fs::read_to_string(&module).context("Failed to read module content")?;
                let doc_path = Some(PathBuf::from(&module).with_extension("md"));
                if self.offline {
                    generate_docs_offline(&module.to_string(), &module_content, doc_path).await.context("Failed to generate documentation offline")?;
                } else {
                    generate_docs(&module.to_string(), &module_content, doc_path).await.context("Failed to generate documentation")?;
                }
            }
        }
        
        Ok(())
    }
}

pub fn get_head_commit_hash(url: &str) -> Result<String> {
    let github_url = if url.starts_with("https://github.com/") {
        url.to_string()
    } else {
        format!("https://github.com/{}", url)
    };

    let (repo_url, _) = github_url.rsplit_once("/blob/").unwrap_or((&github_url, ""));

    let output = Command::new("git")
        .args(["ls-remote", repo_url, "HEAD"])
        .output()?;

    if output.status.success() {
        let stdout = String::from_utf8(output.stdout)?;
        let hash = stdout.split_whitespace().next().unwrap_or("").to_string();
        if !hash.is_empty() {
            Ok(hash[..7].to_string())  // Return only the first 7 characters (short hash)
        } else {
            Err(anyhow::anyhow!("Failed to get HEAD commit hash: Empty hash returned"))
        }
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(anyhow::anyhow!("Failed to get HEAD commit hash: {}", stderr))
    }
}

fn include_entire_repo(url: &str, tmp_path: &PathBuf, riscv: bool, commit_hash: Option<&str>, is_head: bool) -> Result<HashSet<String>> {
    let url = format!("https://github.com/{}", url);
    println!("Full GitHub URL: {}@{}", url, commit_hash.unwrap_or("HEAD"));
    include_repo_from_url(&url, "/tmp/", commit_hash, is_head)?;
    add_dependency(&url)?;

    let files = get_files(&tmp_path.to_str().unwrap_or_default());
    let items = get_relative_paths(&files, tmp_path);

    let selected_items = select_modules(&items).map_err(|e| anyhow::anyhow!("{}", e))?;

    process_selected_modules(&url, tmp_path, &selected_items, riscv, commit_hash, is_head)?;

    fs::remove_dir_all(tmp_path)?;
    print_success_message(&url, &selected_items);


    let mut included_modules = HashSet::new();
    for item in selected_items {
        let module_name_with_extension = item.split('/').last().unwrap_or(&item);
        let module_name = module_name_with_extension.split('.').next().unwrap_or(module_name_with_extension);
        let module_path = format!("vpm_modules/{module_name}/rtl/{module_name_with_extension}");
        included_modules.insert(module_path);
    }

    Ok(included_modules)
}

fn include_single_module(url: &str, riscv: bool, commit_hash: Option<&str>, is_head: bool) -> Result<HashSet<String>> {
    let repo_url = get_github_repo_url(url).unwrap();
    include_repo_from_url(&repo_url, "/tmp/", commit_hash, is_head)?;
    add_dependency(&repo_url)?;
    println!("Repo URL: {}@{}", repo_url, commit_hash.unwrap_or("HEAD"));
    let module_path = get_component_path_from_github_url(url).unwrap_or_default();
    println!("Including module: {}", module_path);
    include_module_from_url(&module_path, &repo_url, riscv, commit_hash, is_head)?;
    println!("Successfully installed module: {}", module_path);

    let mut included_modules = HashSet::new(); 
    let module_name_with_extension = module_path.split('/').last().unwrap_or(&module_path);
    let module_name = module_name_with_extension.split('.').next().unwrap_or(module_name_with_extension);
    let local_module_path = format!("vpm_modules/{module_name}/rtl/{module_name_with_extension}");
    included_modules.insert(local_module_path);
    Ok(included_modules)
}

fn get_files(directory: &str) -> Vec<String> {
    WalkDir::new(directory)
        .into_iter()
        .filter_map(|entry| {
            entry.ok().and_then(|e| {
                if e.file_type().is_file() {
                    Some(e.path().to_string_lossy().into_owned())
                } else {
                    None
                }
            })
        })
        .collect()
}

fn get_relative_paths(files: &[String], tmp_path: &PathBuf) -> Vec<String> {
    files.iter()
        .map(|file| file.strip_prefix(&tmp_path.to_string_lossy().as_ref())
            .unwrap_or(file)
            .trim_start_matches('/')
            .to_string())
        .collect()
}

fn select_modules(items: &[String]) -> Result<HashSet<String>, Box<dyn std::error::Error>> {
    fn filter_items<'a>(items: &'a [String], matcher: &SkimMatcherV2, query: &str) -> Vec<&'a String> {
        let mut filtered: Vec<&String> = items
            .iter()
            .filter(|item| item.ends_with(".v") || item.ends_with(".sv"))
            .collect();
    
        if !query.is_empty() {
            filtered = filtered
                .into_iter()
                .filter(|&item| matcher.fuzzy_match(item, query).is_some())
                .collect();
        }
    
        filtered
    }

    let matcher = SkimMatcherV2::default();
    let mut selected_items: HashSet<String> = HashSet::new();

    // Initialize terminal
    let mut stdout = io::stdout();
    enable_raw_mode()?;
    execute!(
        stdout,
        EnterAlternateScreen,
        EnableMouseCapture,
        Hide
    )?;

    let mut query = String::new();
    let mut filtered_items = filter_items(items, &matcher, &query);
    let mut current_selection: usize = 0;

    loop {
        // Clear the screen and reset cursor position
        execute!(stdout, MoveTo(0, 0), Clear(ClearType::All))?;

        // Display the prompt and query
        write!(
            stdout,
            "Enter module name (or press Esc to finish): {}\r\n",
            query
        )?;
        if !selected_items.is_empty() {
            write!(stdout, "Selected modules:\r\n{}\r\n\r\n", selected_items.iter().map(|item| format!("\t- {}", item)).collect::<Vec<String>>().join("\r\n"))?;
        }

        // Get terminal size
        let (_, height) = crossterm::terminal::size()?;
        let max_display = (height - 4 - selected_items.len() as u16) as usize; // Subtract 3 for prompt and margins

        // Display the filtered items with selection highlighting
        for (index, item) in filtered_items.iter().take(max_display).enumerate() {
            if index == current_selection {
                execute!(
                    stdout,
                    SetBackgroundColor(Color::Blue),
                    SetForegroundColor(Color::White),
                    Print(format!("> {}\r\n", item)),
                    ResetColor
                )?;
            } else {
                write!(stdout, "  {}\r\n", item)?;
            }
        }

        stdout.flush()?;

        // Wait for user input
        if let Event::Key(KeyEvent { code, .. }) = event::read()? {
            match code {
                KeyCode::Char(c) => {
                    query.push(c);
                    filtered_items = filter_items(items, &matcher, &query);
                    current_selection = 0;
                }
                KeyCode::Backspace => {
                    query.pop();
                    filtered_items = filter_items(items, &matcher, &query);
                    current_selection = 0;
                }
                KeyCode::Up => {
                    if current_selection > 0 {
                        current_selection -= 1;
                    }
                }
                KeyCode::Down => {
                    if current_selection + 1 < filtered_items.len().min(max_display) {
                        current_selection += 1;
                    }
                }
                KeyCode::Enter => {
                    if !filtered_items.is_empty() {
                        let selected = filtered_items[current_selection].clone();
                        selected_items.insert(selected.to_string());

                        query.clear();
                        filtered_items = filter_items(items, &matcher, &query);
                        current_selection = 0;
                        continue;
                    }
                }
                KeyCode::Esc => {
                    break;
                }
                _ => {}
            }
        }
    }

    // Restore terminal
    execute!(
        stdout,
        LeaveAlternateScreen,
        DisableMouseCapture,
        Show
    )?;
    disable_raw_mode()?;

    Ok(selected_items)
}

fn process_selected_modules(url: &str, tmp_path: &PathBuf, selected_items: &HashSet<String>, riscv: bool, commit_hash: Option<&str>, is_head: bool) -> Result<()> {
    for item in selected_items {
        let displayed_path = item.strip_prefix(tmp_path.to_string_lossy().as_ref()).unwrap_or(item).trim_start_matches('/');
        println!("Including module: {}", displayed_path);
        
        let full_path = tmp_path.join(displayed_path);
        let module_path = full_path.strip_prefix(tmp_path).unwrap_or(&full_path).to_str().unwrap().trim_start_matches('/');
        println!("Module path: {}", module_path);

        include_module_from_url(module_path, url, riscv, commit_hash, is_head)?;
    }

    if selected_items.is_empty() {
        println!("No modules selected. Including entire repository.");
        include_repo_from_url(url, "./vpm_modules/", commit_hash, is_head)?;
    }

    Ok(())
}

fn print_success_message(url: &str, selected_items: &HashSet<String>) {
    if !selected_items.is_empty() {
        let installed_modules = selected_items.iter()
            .map(|item| item.to_string())
            .collect::<Vec<String>>()
            .join(", ");
        println!("Successfully installed module(s): {}", installed_modules);
    } else {
        println!("Successfully installed repository '{}'.", name_from_url(url));
    }
}

fn name_from_url(url: &str) -> &str {
    url.rsplit('/').find(|&s| !s.is_empty()).unwrap_or_default()
}

fn get_component_path_from_github_url(url: &str) -> Option<String> {
    let parts: Vec<&str> = url.split("/").collect();
    if parts.len() < 8 || !url.starts_with("https://github.com/") {
        return None;
    }

    Some(parts[7..].join("/"))
}

fn get_github_repo_url(url: &str) -> Option<String> {
    let parts: Vec<&str> = url.split('/').collect();
    if parts.len() < 5 || !url.starts_with("https://github.com/") {
        return None;
    }

    Some(format!("https://github.com/{}/{}", parts[3], parts[4]))
}

fn is_full_filepath(path: &str) -> bool {
    path.contains('/') || path.contains('\\')
}

fn filepath_to_dir_entry(filepath: PathBuf) -> Result<DirEntry> {
    WalkDir::new(filepath)
        .min_depth(0)
        .max_depth(0)
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("Failed to create DirEntry"))?
        .context("Failed to create DirEntry")
}

fn generate_top_v_content(module_path: &str) -> Result<String> {
    println!("Generating top.v file for RISC-V in {}", module_path);
    let module_content = fs::read_to_string(module_path)?;

    let mut top_content = String::new();
    top_content.push_str("// Auto-generated top.v file for RISC-V\n\n");

    // Use regex to find module declaration
    let module_re = regex::Regex::new(r"module\s+(\w+)\s*(?:#\s*\(([\s\S]*?)\))?\s*\(([\s\S]*?)\);").unwrap();
    if let Some(captures) = module_re.captures(&module_content) {
        let module_name = captures.get(1).unwrap().as_str();
        println!("Module name: {}", module_name);

        // Extract parameters
        let params = captures.get(2).map_or(Vec::new(), |m| {
            m.as_str().lines()
                .map(|line| line.trim())
                .filter(|line| !line.is_empty())
                .collect()
        });

        // Extract ports
        let ports: Vec<&str> = captures.get(3).unwrap().as_str()
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect();

        // Generate top module ports
        top_content.push_str("module top (\n");
        for port in &ports {
            top_content.push_str(&format!("    {}\n", port));
        }
        top_content.push_str(");\n\n");

        // Instantiate the module
        top_content.push_str(&format!("{} #(\n", module_name));
        for param in params.iter() {
            if let Some((name, value)) = param.split_once('=') {
                let name = name.trim().trim_start_matches("parameter").trim();
                let name = name.split_whitespace().last().unwrap_or(name);
                let value = value.trim().trim_end_matches(',');
                top_content.push_str(&format!("    .{}({}),\n", name, value));
            }
        }
        top_content.push_str(") cpu (\n");

        // Connect ports
        let port_re = regex::Regex::new(r"(input|output|inout)\s+(?:wire|reg)?\s*(?:\[.*?\])?\s*(\w+)").unwrap();
        for (i, port) in ports.iter().enumerate() {
            if let Some(port_captures) = port_re.captures(port) {
                let port_name = port_captures.get(2).unwrap().as_str();
                top_content.push_str(&format!("    .{}({}){}\n", port_name, port_name, if i < ports.len() - 1 { "," } else { "" }));
            }
        }
        top_content.push_str(");\n\n");

        top_content.push_str("endmodule\n");
        return Ok(top_content);
    }

    Err(anyhow::anyhow!("No module declaration found in the file"))
}

fn generate_xdc_content(module_path: &str) -> Result<String> {
    println!("Generating constraints.xdc file for Xilinx Artix-7 board in {}", module_path);
    let module_content = fs::read_to_string(module_path)?;

    let mut xdc_content = String::new();
    xdc_content.push_str("## Auto-generated constraints.xdc file for Xilinx Artix-7 board\n\n");

    // Use regex to find all ports
    let port_re = regex::Regex::new(r"(?m)^\s*(input|output|inout)\s+(?:wire|reg)?\s*(?:\[.*?\])?\s*(\w+)").unwrap();
    let mut ports = Vec::new();

    for captures in port_re.captures_iter(&module_content) {
        let port_type = captures.get(1).unwrap().as_str();
        let port_name = captures.get(2).unwrap().as_str();
        ports.push((port_type, port_name));
    }

    // Define pin mappings (you may need to adjust these based on your specific board)
    let pin_mappings = [
        ("clk", "E3"),
        ("resetn", "C12"),
        ("trap", "D10"),
        ("mem_valid", "C11"),
        ("mem_instr", "C10"),
        ("mem_ready", "A10"),
        ("mem_addr[0]", "A8"),
        ("mem_wdata[0]", "C5"),
        ("mem_wstrb[0]", "C6"),
        ("mem_rdata[0]", "D5"),
    ];

    // Generate constraints for each port
    for (_port_type, port_name) in ports {
        if let Some((_, pin)) = pin_mappings.iter().find(|&&(p, _)| p == port_name) {
            let iostandard = if port_name == "clk" { "LVCMOS33" } else { "LVCMOS33" };
            xdc_content.push_str(&format!("set_property -dict {{ PACKAGE_PIN {} IOSTANDARD {} }} [get_ports {{ {} }}]\n", pin, iostandard, port_name));
        } else {
            println!("Warning: No pin mapping found for port: {}", port_name);
        }
    }

    // Add clock constraint
    if let Some((_, _clk_pin)) = pin_mappings.iter().find(|&&(p, _)| p == "clk") {
        xdc_content.push_str(&format!("\n## Clock signal\n"));
        xdc_content.push_str(&format!("create_clock -period 10.000 -name sys_clk_pin -waveform {{0.000 5.000}} -add [get_ports {{ clk }}]\n"));
    } else {
        println!("Warning: No clock signal found. XDC file may be incomplete.");
        xdc_content.push_str("\n## Warning: No clock signal found. Please add clock constraints manually.\n");
    }

    Ok(xdc_content)
}

pub fn include_module_from_url(module_path: &str, url: &str, riscv: bool, commit_hash: Option<&str>, is_head: bool) -> Result<()> {
    let package_name = name_from_url(url);

    include_repo_from_url(url, "/tmp/", commit_hash, is_head)?;
    let module_name = Path::new(module_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(module_path);
    let destination = format!("./vpm_modules/{}/rtl", module_name);
    fs::create_dir_all(&destination)?;
    process_module(package_name, module_path, destination.to_owned(), &mut HashSet::new(), url, true, commit_hash)?;

    let module_path = Path::new(&destination).join(Path::new(module_path).file_name().unwrap());
    anyhow::ensure!(module_path.exists(), "Module file not found in the destination folder");

    if riscv {
        let top_v_content = generate_top_v_content(&module_path.to_str().unwrap())?;
        fs::write(format!("{}/top.v", destination), top_v_content)?;
        println!("Created top.v file for RISC-V in {}", destination);
        // Generate .xdc file for Xilinx Artix-7 board
        let xdc_content = generate_xdc_content(&format!("{}/top.v", destination))?;
        fs::write(format!("{}/constraints.xdc", destination), xdc_content)?;
        println!("Created constraints.xdc file for Xilinx Artix-7 board in {}", destination);
    }
    add_top_module(url, current_dir()?.join(module_path.file_name().unwrap()).to_str().unwrap(), commit_hash.unwrap_or(""))?;
    
    Ok(())
}

pub fn process_module(package_name: &str, module: &str, destination: String, visited: &mut HashSet<String>, url: &str, is_top_module: bool, commit_hash: Option<&str>) -> Result<HashSet<String>> {
    // println!("Processing module: {}", module);
    let module_name = module.strip_suffix(".v").or_else(|| module.strip_suffix(".sv")).unwrap_or(module);
    let module_with_ext = if module.ends_with(".v") || module.ends_with(".sv") {
        module.to_string()
    } else {
        format!("{}.v", module_name)
    };
    if !visited.insert(module_with_ext.clone()) {
        return Ok(HashSet::new());
    }

    let tmp_path = PathBuf::from("/tmp").join(package_name);
    let file_path = tmp_path.join(&module_with_ext);

    let target_path = PathBuf::from(&destination);

    println!("Including submodule '{}'", module_with_ext);

    let mut processed_modules = HashSet::new();

    if is_full_filepath(&module_with_ext) {
        // println!("Full filepath detected for module '{}'", module_with_ext);
        let dir_entry = filepath_to_dir_entry(file_path)?;
        // println!("Dir entry: {}", dir_entry.path().display());
        process_file(&dir_entry, &target_path.to_str().unwrap(), module, url, visited, is_top_module)?;
        processed_modules.insert(module_with_ext.clone());
    } else {
        // println!("Full filepath not detected for module '{}'", module_with_ext);
        process_non_full_filepath(module_name, &tmp_path, &target_path, url, visited, is_top_module, &mut processed_modules)?;
    }

    let submodules = download_and_process_submodules(package_name, module, &destination, url, visited, is_top_module, commit_hash)?;
    processed_modules.extend(submodules);

    Ok(processed_modules)
}

fn process_non_full_filepath(module_name: &str, tmp_path: &PathBuf, target_path: &PathBuf, url: &str, visited: &mut HashSet<String>, is_top_module: bool, processed_modules: &mut HashSet<String>) -> Result<()> {
    let matching_entries = find_matching_entries(module_name, tmp_path);
    println!("Found {} matching entries for module '{}'", matching_entries.len(), module_name);
    if matching_entries.is_empty() {
        println!("No matching files found for module '{}'. Skipping...", module_name);
    } else if matching_entries.len() == 1 {
        let dir_entry = filepath_to_dir_entry(matching_entries[0].clone())?;
        process_file(&dir_entry, target_path.to_str().unwrap(), module_name, url, visited, is_top_module)?;
        processed_modules.insert(format!("{}.v", module_name));
    } else {
        process_multiple_matches(matching_entries, target_path, module_name, url, visited, is_top_module, processed_modules)?;
    }

    Ok(())
}

fn find_matching_entries(module_name: &str, tmp_path: &PathBuf) -> Vec<PathBuf> {
    WalkDir::new(tmp_path)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| {
            entry.file_name().to_str() == Some(&format!("{}.sv", module_name)) || 
            entry.file_name().to_str() == Some(&format!("{}.v", module_name))
        })
        .map(|entry| entry.path().to_path_buf())
        .collect()
}

fn process_multiple_matches(matching_entries: Vec<PathBuf>, target_path: &PathBuf, module_name: &str, url: &str, visited: &mut HashSet<String>, is_top_module: bool, processed_modules: &mut HashSet<String>) -> Result<()> {
    println!("Multiple modules found for '{}'. Please choose:", module_name);
    for (i, entry) in matching_entries.iter().enumerate() {
        println!("{}: {}", i + 1, entry.display());
    }

    let mut choice = String::new();
    std::io::stdin().read_line(&mut choice)?;
    let index: usize = choice.trim().parse()?;

    if index > 0 && index <= matching_entries.len() {
        let dir_entry = filepath_to_dir_entry(matching_entries[index - 1].clone())?;
        process_file(&dir_entry, target_path.to_str().unwrap(), module_name, url, visited, is_top_module)?;
        processed_modules.insert(format!("{}.v", module_name));
    } else {
        anyhow::bail!("Invalid choice");
    }

    Ok(())
}

fn process_file(entry: &DirEntry, destination: &str, module_path: &str, url: &str, visited: &mut HashSet<String>, is_top_module: bool) -> Result<()> {
    let target_path = PathBuf::from(destination);
    let extension = entry.path().extension().and_then(|s| s.to_str()).unwrap_or("v");
    fs::copy(entry.path(), &target_path.join(entry.file_name()))?;

    let contents = fs::read_to_string(entry.path())?;
    let header_content = generate_headers(&contents)?;
    let module_name = Path::new(module_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(module_path);
    
    let module_name_with_ext = if !module_name.ends_with(".v") && !module_name.ends_with(".sv") {
        format!("{}.{}", module_name, extension)
    } else {
        module_name.to_string()
    };
    let header_filename = format!("{}.{}", module_name.strip_suffix(".v").unwrap_or(module_name), if extension == "sv" { "svh" } else { "vh" });
    fs::write(target_path.join(&header_filename), header_content)?;
    println!("Generating header file: {}", target_path.join(&header_filename).to_str().unwrap());

    let full_module_path = target_path.join(&module_name_with_ext);
    update_lockfile(&full_module_path, url, &contents, visited, is_top_module)?;

    Ok(())
}

fn download_and_process_submodules(package_name: &str, module_path: &str, destination: &str, url: &str, visited: &mut HashSet<String>, _is_top_module: bool, commit_hash: Option<&str>) -> Result<HashSet<String>> {
    let module_name = Path::new(module_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(module_path);
    // println!("Processing submodule: {}", module_path);
    let module_name_with_ext = if module_path.ends_with(".sv") {
        format!("{}.sv", module_name)
    } else if module_path.ends_with(".v") {
        format!("{}.v", module_name)
    } else {
        module_path.to_string()
    };

    let full_module_path = PathBuf::from(destination).join(&module_name_with_ext);
    // println!("Full module path: {}", full_module_path.display());
    let contents = match fs::read_to_string(&full_module_path) {
        Ok(c) => c,
        Err(e) => {
            println!("Warning: Failed to read file {}: {}. Skipping this module.", full_module_path.display(), e);
            return Ok(HashSet::new());
        }
    };

    let submodules = match get_submodules(&contents) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Warning: Failed to get submodules from {}: {}. Continuing without submodules.", full_module_path.display(), e);
            HashSet::new()
        }
    };

    let mut all_submodules = HashSet::new();

    for submodule in submodules {
        let submodule_with_ext = if submodule.ends_with(".v") || submodule.ends_with(".sv") {
            submodule.to_string()
        } else {
            let parent_extension = Path::new(&module_name_with_ext)
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("v");
            format!("{}.{}", &submodule, parent_extension)
        };
        if !visited.contains(&submodule_with_ext) {
            let submodule_destination = PathBuf::from(destination);
            if let Err(e) = fs::create_dir_all(&submodule_destination) {
                eprintln!("Warning: Failed to create directory {}: {}. Skipping this submodule.", submodule_destination.display(), e);
                continue;
            }
            
            match process_module(
                package_name,
                &submodule_with_ext,
                submodule_destination.to_str().unwrap().to_string(),
                visited,
                &url,
                false,
                commit_hash.clone()
            ) {
                Ok(processed_submodules) => {
                    all_submodules.insert(submodule_with_ext.clone());
                    all_submodules.extend(processed_submodules);
                },
                Err(e) => {
                    eprintln!("Warning: Failed to process submodule {}: {}. Skipping this submodule.", submodule_with_ext, e);
                    continue;
                }
            }

            let full_submodule_path = submodule_destination.join(&submodule_with_ext);
            if let Err(e) = update_lockfile(&full_submodule_path, &url, &contents, visited, false) {
                eprintln!("Warning: Failed to update lockfile for {}: {}. Continuing without updating lockfile.", full_submodule_path.display(), e);
            }
        }
    }

    Ok(all_submodules)
}

fn update_lockfile(full_path: &PathBuf, url: &str, contents: &str, visited: &HashSet<String>, is_top_module: bool) -> Result<()> {
    let mut lockfile = fs::read_to_string("vpm.lock").unwrap_or_default();
    let module_entry = if is_top_module {
        format!("[[package]]\nfull_path = \"{}\"\nsource = \"{}\"\nparents = []\n", full_path.display(), url)
    } else {
        format!("[[package]]\nfull_path = \"{}\"\nsource = \"{}\"\n", full_path.display(), url)
    };

    let submodules = get_submodules(contents)?;
    let submodules_vec: Vec<String> = submodules.into_iter().collect();

    if !lockfile.contains(&format!("full_path = \"{}\"", full_path.display())) {
        let formatted_submodules = submodules_vec.iter()
            .map(|s| format!("  \"{}\",", s))
            .collect::<Vec<_>>()
            .join("\n");
        lockfile.push_str(&format!("\n{}\nsubmodules = [\n{}\n]\n", module_entry, formatted_submodules));
    } else {
        update_submodules(&mut lockfile, &module_entry, &submodules_vec);
    }

    for submodule in &submodules_vec {
        if !visited.contains(submodule) {
            let submodule_path = full_path.parent().unwrap().join(submodule);
            if let Some(existing_entry) = lockfile.find(&format!("\n[[package]]\nfull_path = \"{}\"", submodule_path.display())) {
                let parent_start = lockfile[existing_entry..].find("parents = [").map(|i| existing_entry + i);
                if let Some(start) = parent_start {
                    let end = lockfile[start..].find(']').map(|i| start + i + 1).unwrap_or(lockfile.len());
                    let current_parents = lockfile[start..end].to_string();
                    let new_parents = if current_parents.contains(&full_path.display().to_string()) {
                        current_parents
                    } else {
                        format!("{}  \"{}\",\n]", &current_parents[..current_parents.len() - 1], full_path.display())
                    };
                    lockfile.replace_range(start..end, &new_parents);
                }
            } else {
                let submodule_entry = format!("\n[[package]]\nfull_path = \"{}\"\nsource = \"{}\"\nparents = [\n  \"{}\",\n]\nsubmodules = []\n", submodule_path.display(), url, full_path.display());
                lockfile.push_str(&submodule_entry);
            }
        }
    }

    fs::write("vpm.lock", lockfile)?;
    Ok(())
}

fn update_submodules(lockfile: &mut String, module_entry: &str, submodules: &[String]) {
    if let Some(start) = lockfile.find(module_entry).and_then(|pos| lockfile[pos..].find("submodules = [").map(|offset| pos + offset)) {
        let end = lockfile[start..].find(']').map(|pos| start + pos + 1).unwrap_or(lockfile.len());
        let new_modules = format!("submodules = [\n{}\n]", submodules.iter().map(|m| format!("  \"{}\",", m)).collect::<Vec<_>>().join("\n"));
        lockfile.replace_range(start..end, &new_modules);
    }
}

pub fn include_repo_from_url(url: &str, location: &str, commit_hash: Option<&str>, is_head: bool) -> Result<()> {
    let repo_path = Path::new(location).join(name_from_url(url));
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner().template("{spinner} {msg}").unwrap());
    pb.set_message("Reading repository...");
    pb.enable_steady_tick(std::time::Duration::from_millis(100));
    clone_repo(url, &repo_path, commit_hash, is_head)?;
    pb.finish_with_message("Reading repository complete");
    Ok(())
}

pub fn clone_repo(url: &str, repo_path: &Path, commit_hash: Option<&str>, is_head: bool) -> Result<()> {
    if repo_path.exists() {
        fs::remove_dir_all(repo_path)?;
    }
    Command::new("git")
        .args([ "clone", "--depth", "1", "--single-branch", "--jobs", "4",
            url, repo_path.to_str().unwrap_or_default(),
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .with_context(|| format!("Failed to clone repository from URL: '{}'", url))?;

    println!("Cloned repository: {}", repo_path.to_str().unwrap_or_default());
    if !is_head {
        if let Some(hash) = commit_hash {
            Command::new("git")
            .args([ "-C", repo_path.to_str().unwrap_or_default(), "checkout", hash ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .with_context(|| format!("Failed to checkout commit hash: '{}'", hash))?;
        }
    }
    Ok(())
}