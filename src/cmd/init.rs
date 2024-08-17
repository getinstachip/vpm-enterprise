use anyhow::{Context, Result};
use std::env;

use crate::cmd::{Execute, Init};
use crate::embedding::{create_client, create_index, embed_library, insert_documents};
use rand::Rng;


impl Execute for Init {
    async fn execute(&self) -> Result<()> {
        println!("Initializing...");
        embed_codebase().await?;
        Ok(())
    }
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
    let current_dir = env::current_dir().unwrap();
    println!("Current directory: {:?}", current_dir);
    let documents = embed_library(&current_dir, &index_name).await.unwrap();
    println!("Number of embedded documents: {}", documents.len());
    insert_documents(&index_name, &documents)
        .await
        .unwrap();
    println!("Codebase embedded and stored successfully!");
    Ok(index_name)
}