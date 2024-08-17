use async_openai::config::OpenAIConfig;
use async_openai::{types::CreateEmbeddingRequestArgs, Client};
use elasticsearch::http::headers::{HeaderMap, HeaderValue, CONTENT_LENGTH};
use elasticsearch::{
    http::transport::{SingleNodeConnectionPool, TransportBuilder},
    indices::IndicesCreateParts,
    Elasticsearch
};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use reqwest::Client as ReqwestClient;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use anyhow::anyhow;
use elasticsearch::SearchParts;

pub const ES_URL: &str = "https://f563c8312d3e4148831bd9ac31bcb040.us-central1.gcp.cloud.es.io:443";
pub const ES_API_KEY: &str = "TU5RSVRwRUI2b2hUcmhzWlRoR286WGFDOHBIek5RT1NxODN2RTgyQ3VhQQ==";
use anyhow::{Context, Result};

pub(crate) fn create_client() -> Result<Elasticsearch> {
    let conn_pool = SingleNodeConnectionPool::new(ES_URL.parse().context("Failed to parse ES_URL")?);
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("ApiKey {}", ES_API_KEY))
            .context("Failed to create Authorization header")?,
    );
    let transport = TransportBuilder::new(conn_pool)
        .headers(headers)
        .build()
        .context("Failed to build transport")?;
    let client = Elasticsearch::new(transport);
    Ok(client)
}

pub(crate) async fn generate_embedding(
    code_snippet: &str,
) -> anyhow::Result<Vec<f32>> {
    let api_key = std::env::var("OPENAI_API_KEY").context("OPENAI_API_KEY must be set")?;
    let openai_config = OpenAIConfig::new().with_api_key(api_key);
    let client = Client::with_config(openai_config);
    // Print length of code snippet
    // println!("Length of code snippet: {} characters", code_snippet.len());
    let response = client
        .embeddings()
        .create(
            CreateEmbeddingRequestArgs::default()
                .model("text-embedding-ada-002")
                .input(code_snippet)
                .build()?,
        )
        .await
        .context("Failed to generate embedding")?;

    if response.data.is_empty() {
        return Err(anyhow!("No embedding data returned"));
    }

    Ok(response.data[0].embedding.clone())
}

async fn create_document(
    file_path: &str,
    code_chunk: &str,
) -> anyhow::Result<HashMap<String, Value>> {
    let embedding = generate_embedding(code_chunk).await.context("Failed to generate embedding")?;
    Ok(HashMap::from([
        (
            "file_path".to_string(),
            Value::String(file_path.to_string()),
        ),
        ("code".to_string(), Value::String(code_chunk.to_string())),
        ("embedding".to_string(), json!(embedding)),
    ]))
}

pub(crate) async fn create_index(
    client: &Elasticsearch,
    index_name: &str,
) -> anyhow::Result<()> {
    let body = json!({
        "settings": {
            "number_of_shards": 1,
            "number_of_replicas": 1
        },
        "mappings": {
            "properties": {
                "file_path": {"type": "keyword"},
                "code": {"type": "text"},
                "embedding": {
                    "type": "dense_vector",
                    "dims": 1536,
                    "index": true,
                    "similarity": "cosine"
                }
            }
        }
    });

    let body_string = serde_json::to_string(&body).context("Failed to serialize JSON body")?;
    let body_length = body_string.len();

    client
        .indices()
        .create(IndicesCreateParts::Index(index_name))
        .header(
            CONTENT_LENGTH,
            HeaderValue::from_str(&body_length.to_string())
                .context("Failed to create Content-Length header")?,
        )
        .body(body)
        .send()
        .await
        .context("Failed to send create index request")?;

    Ok(())
}

pub(crate) async fn insert_documents(
    index_name: &str,
    embedded_documents: &[Value],
) -> anyhow::Result<()> {
    let client = ReqwestClient::new();
    for chunk in embedded_documents.chunks(2) {
        if chunk.len() == 2 {
            let doc_id = chunk[0]["index"]["_id"].as_str();
            let doc_body = &chunk[1];

            let url = match doc_id {
                Some(id) => format!("{}/{}/_doc/{}", ES_URL, index_name, id),
                None => format!("{}/{}/_doc", ES_URL, index_name),
            };
            let mut headers = HeaderMap::new();
            headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&format!("ApiKey {}", ES_API_KEY))
                    .context("Failed to create Authorization header")?,
            );
            let request = if doc_id.is_some() {
                client.put(&url)
            } else {
                client.post(&url)
            };
            request.headers(headers).json(doc_body).send().await
                .context("Failed to send request to Elasticsearch")?;
        }
    }
    Ok(())
}

fn chunk_content(content: &str, chunk_size: usize) -> Vec<String> {
    content.chars()
        .collect::<Vec<char>>()
        .chunks(chunk_size)
        .map(|chunk| chunk.iter().collect::<String>())
        .collect()
}

pub(crate) async fn embed_library(
    fpath: &Path,
    index: &str,
) -> anyhow::Result<Vec<Value>> {
    let mut documents = Vec::new();
    let mut idx = 0;
    for entry in walkdir::WalkDir::new(fpath)
        .into_iter()
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if path.is_file() && path.extension()
            .and_then(|s| s.to_str())
            .map(|ext| ext == "v" || ext == "vhdl" || ext == "vhd")
            .unwrap_or(false)
        {
            let module_code = fs::read_to_string(&path).context("Failed to read file")?;
            let chunks = chunk_content(&module_code, 2048);
            let file_path = path.file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| anyhow!("Invalid file name"))?
                .to_string();

            for (chunk_idx, chunk) in chunks.iter().enumerate() {
                let doc = create_document(&file_path, chunk).await.context("Failed to create document")?;
                documents.push(json!({
                    "index": {
                        "_index": index,
                        "_id": format!("{}-{}", idx, chunk_idx)
                    }
                }));
                // println!("Processing file: {}", file_path);
                documents.push(json!({
                    "file_path": doc["file_path"],
                    "code": doc["code"],
                    "embedding": doc["embedding"],
                }));
            }
            idx += 1;
        }
    }
    Ok(documents)
}


pub(crate) async fn vector_search(
    client: &Elasticsearch,
    index_name: &str,
    query_vector: Vec<f32>,
    top_k: usize,
) -> anyhow::Result<Vec<String>> {
    let search_query = json!({
        "size": top_k,
        "query": {
            "script_score": {
                "query": {"match_all": {}},
                "script": {
                    "source": "cosineSimilarity(params.query_vector, 'embedding') + 1.0",
                    "params": {"query_vector": query_vector}
                }
            }
        }
    });

    let response = client
        .search(SearchParts::Index(&[index_name]))
        .body(search_query)
        .send()
        .await
        .context("Failed to send search request")?;

    let mut results = Vec::new();
    if let Some(hits) = response.json::<Value>().await
        .context("Failed to parse response JSON")?["hits"]["hits"].as_array() {
        for hit in hits {
            if let Some(code) = hit["_source"]["code"].as_str() {
                results.push(code.to_string());
            }
        }
    }
    Ok(results)
}
