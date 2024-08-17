use anyhow::{Context, Result};
use crate::cmd::{Execute, Chipyard};
use std::{fs, process::Command};
use clust::Client;
use clust::messages::{Message, MessagesRequestBody, ClaudeModel, MaxTokens};
use rand::Rng;

use crate::embedding::{create_client, create_index, insert_documents, vector_search, generate_embedding, embed_library};

impl Execute for Chipyard {
    async fn execute(&self) -> Result<()> {
        println!("Building configuration: '{}'", &self.config_name);
        init(&self.config_name).await?;
        Ok(())
    }
}

async fn init(config: &str) -> Result<()> {
    // clone_repo("https://github.com/ucb-bar/chipyard.git", "chipyard")?;
    std::env::set_current_dir("chipyard")?;
    
    Command::new("./scripts/init-submodules-no-riscv-tools.sh")
        .status()
        .with_context(|| "Failed to initialize submodules")?;

    Command::new("./scripts/build-toolchains.sh riscv-tools")
        .status()
        .with_context(|| "Failed to build toolchains")?;

    std::env::set_current_dir("sims/verilator")?;

    Command::new(format!("make CONFIG={} verilog", config))
        .status()
        .with_context(|| "Failed to generate Verilog")?;

    let index_name = embed_codebase().await?;
    let verilog_file = create_top_module(config, &index_name).await?;
    generate_testbench(&verilog_file)?;

    Ok(())
}

async fn embed_codebase() -> Result<String> {
    println!("Performing flex install: Embedding and storing codebase...");
    let es_client = create_client().context("Failed to create Elasticsearch client")?;
    let random_string: String = (0..3)
        .map(|_| rand::thread_rng().sample(rand::distributions::Alphanumeric) as char)
        .collect();
    let index_name = format!("codebase{}", random_string).to_lowercase();
    println!("Creating index: {}", index_name);
    create_index(&es_client, &index_name)
        .await
        .with_context(|| format!("Failed to create index '{}'", index_name))?;
    println!("Index '{}' created successfully", index_name);
    let current_dir = std::env::current_dir().unwrap();
    println!("Current directory: {:?}", current_dir);
    let documents = embed_library(&current_dir, &index_name).await.unwrap();
    println!("Number of embedded documents: {}", documents.len());
    insert_documents(&index_name, &documents)
        .await
        .unwrap();
    println!("Codebase embedded and stored successfully!");
    Ok(index_name)
}

async fn create_top_module(config: &str, index_name: &str) -> Result<String> {
    let top_module_name = format!("ChipTop_{}", config);
    let verilog_file = format!("{}.sv", top_module_name);
    let client = Client::from_env().expect("API key not found in environment");

    // Perform semantic search on the embedded codebase
    let es_client = create_client().context("Failed to create Elasticsearch client")?;
    let query = "top module connections rocket chip";
    let embedding = generate_embedding(query).await?;
    let relevant_docs = vector_search(&es_client, index_name, embedding, 5).await?;
    let context = relevant_docs.join("\n\n");

    let prompt = format!(
        "Based on the following context from the existing Verilog codebase and Rocket Chip implementation, generate a top module named '{}' with correct connections:\n\n{}",
        top_module_name, context
    );

    let request_body = MessagesRequestBody {
        model: ClaudeModel::Claude3Sonnet20240229,
        max_tokens: MaxTokens::new(2000, ClaudeModel::Claude3Sonnet20240229).unwrap(),
        messages: vec![
            Message::user(prompt),
        ],
        ..Default::default()
    };

    let response = client.create_a_message(request_body).await?;
    let response_content = response.content.flatten_into_text().unwrap();

    fs::write(&verilog_file, response_content)
        .with_context(|| format!("Failed to create top-level SystemVerilog file: {}", verilog_file))?;

    Ok(verilog_file)
}

fn generate_testbench(file_name: &str) -> Result<()> {
    let module_name = file_name.trim_end_matches(".sv");
    let tb_file_name = format!("{}_tb.sv", module_name);
    let tb_content = format!(
        r#"
`timescale 1ns / 1ps

module tb_{};

    // Clock and reset
    logic clock;
    logic reset;

    // Instantiate the DUT (Design Under Test)
    {} dut (
        .clock(clock),
        .reset(reset)
        // Add other port connections here
    );

    // Clock generation
    always #5 clock = ~clock;

    // Test stimulus
    initial begin
        // Initialize
        clock = 0;
        reset = 1;

        // Reset
        #20 reset = 0;

        // Add your test cases here
        // ...

        // End simulation
        #1000 $finish;
    end

    // Optional: Add assertions, coverage, or other checks

endmodule
"#,
        module_name, module_name
    );

    fs::write(&tb_file_name, tb_content)
        .with_context(|| format!("Failed to create testbench file: {}", tb_file_name))?;

    println!("Generated testbench: {}", tb_file_name);
    Ok(())
}


// fn clone_repo(url: &str, repo_path: &str) -> Result<()> {
//     Command::new("git")
//     .args(["clone", "--depth", "1", "--single-branch", "--jobs", "4", url, repo_path])
//     .status()
//     .with_context(|| format!("Failed to clone repository from URL: '{}'", url))?;

//     Ok(())
// }