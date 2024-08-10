use anyhow::Result;

use crate::cmd::{Execute, Chat};
use clust::Client;
use clust::messages::{Message, MessagesRequestBody, ClaudeModel, MaxTokens};

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
    prompt.push_str("\n\nPlease take the module code and\n\n");
    prompt.push_str(chat_message);
    prompt.push_str("\n\nOnly show me the code output, nothing else.");

    let request_body = MessagesRequestBody {
        model: ClaudeModel::Claude3Sonnet20240229,
        max_tokens: MaxTokens::new(1000, ClaudeModel::Claude3Sonnet20240229).unwrap(),
        messages: vec![
            Message::user(prompt),
        ],
        ..Default::default()
    };
    
    let response = client.create_a_message(request_body).await?;
    let output_file_name = format!("./vpm_modules/arbiter.v/{}_output.v", module_name);
    std::fs::write(&output_file_name, response.content.flatten_into_text().unwrap())?;
    // println!("Response: {}", response.content);

    Ok(())
}
