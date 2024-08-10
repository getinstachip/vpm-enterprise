use anyhow::Result;

use crate::cmd::{Execute, Chat};
use clust::Client;
use clust::messages::{Message, MessagesRequestBody, ClaudeModel, MaxTokens};
use rand::Rng;

impl Execute for Chat {
    async fn execute(&self) -> Result<()> {
        println!("Sending message: '{}' to module: '{}'", &self.message, &self.module_name);
        chat_with_file(&self.message, &self.module_name).await;
        Ok(())
    }
}

async fn chat_with_file(chat_message: &str, module_name: &str) -> Result<()> {
    let client = Client::from_env().expect("API key not found in environment");
    let file_contents = std::fs::read_to_string(format!("./vpm_modules/arbiter.v/{}", module_name)).expect("File not found");
    let mut prompt = format!("This is the module code: {}", file_contents);
    prompt.push_str("\n\nPlease keep this entire code and apply the following tweak: \n\n");
    prompt.push_str(chat_message);
    prompt.push_str("\n\nOnly show me the code output for each request, nothing else. Enclose each output with {code} and {/code}");

    let request_body = MessagesRequestBody {
        model: ClaudeModel::Claude3Sonnet20240229,
        max_tokens: MaxTokens::new(1000, ClaudeModel::Claude3Sonnet20240229).unwrap(),
        messages: vec![
            Message::user(prompt),
        ],
        ..Default::default()
    };
    
    let response = client.create_a_message(request_body).await?;
    let response_content = response.content.flatten_into_text().unwrap();
    split_into_files(response_content, module_name);

    Ok(())
}

fn split_into_files(file_contents: &str, module_name: &str) -> Vec<String> {
    let mut files = Vec::new();
    let mut rng = rand::thread_rng();
    let mut current_file = String::new();
    let mut in_code_block = false;

    for line in file_contents.lines() {
        if line.trim() == "{code}" {
            in_code_block = true;
            current_file.clear();
        } else if line.trim() == "{/code}" {
            in_code_block = false;
            if !current_file.is_empty() {
                files.push(current_file.clone());
            }
        } else if in_code_block {
            current_file.push_str(line);
            current_file.push('\n');
        }
    }

    for (index, file_content) in files.iter().enumerate() {
        let random_string: String = (0..3)
            .map(|_| rng.sample(rand::distributions::Alphanumeric) as char)
            .collect();
        let file_name = format!("./vpm_modules/arbiter.v/{}_{}_{}.v", module_name, index + 1, random_string);
        std::fs::write(&file_name, file_content).unwrap();
    }

    files
}