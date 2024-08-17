use anyhow::Result;

use crate::cmd::{Execute, Tune};
use clust::Client;
use clust::messages::{Message, MessagesRequestBody, ClaudeModel, MaxTokens, SystemPrompt};
use rand::Rng;

impl Execute for Tune {
    async fn execute(&self) -> Result<()> {
        println!("Tuning module: '{}' to module: '{}'", &self.source_module, &self.target_module);
        tune_module(&self.source_module, &self.target_module).await?;
        Ok(())
    }
}

async fn tune_module(source_module: &str, target_module: &str) -> Result<()> {
    let client = Client::from_env().expect("API key not found in environment");
    
    let source_contents = std::fs::read_to_string(format!("./vpm_modules/{}", source_module))
        .expect("Source file not found");
    let target_contents = std::fs::read_to_string(format!("./vpm_modules/{}", target_module))
        .expect("Target file not found");

    let prompt = format!(
        "Your task is to modify the source module to match the pinouts of the target module, enabling seamless integration. Here are the two modules:

Source module:
{}

Target module:
{}

Enclose the output with {{code}} and {{/code}}.",
        source_contents, target_contents
    );

    let system_prompt = SystemPrompt::new("You are an expert Verilog engineer who adds brief comments to explain your changes.");
    let request_body = MessagesRequestBody {
        model: ClaudeModel::Claude3Sonnet20240229,
        max_tokens: MaxTokens::new(4000, ClaudeModel::Claude3Sonnet20240229).unwrap(),
        system: Some(system_prompt),
        messages: vec![
            Message::user(prompt),
        ],
        ..Default::default()
    };
    
    let response = client.create_a_message(request_body).await?;
    let response_content = response.content.flatten_into_text().unwrap();
    
    // Extract the code from the response
    let code_regex = regex::Regex::new(r"\{code\}([\s\S]*?)\{/code\}").unwrap();
    if let Some(captures) = code_regex.captures(&response_content) {
        let modified_code = captures.get(1).unwrap().as_str();
        
        // Generate a unique filename for the tuned module
        let mut rng = rand::thread_rng();
        let random_string: String = (0..3)
            .map(|_| rng.sample(rand::distributions::Alphanumeric) as char)
            .collect();
        let tuned_filename = format!("./vpm_modules/{}_tuned_{}.v", source_module.trim_end_matches(".v"), random_string);
        
        // Write the tuned module to a new file
        std::fs::write(&tuned_filename, modified_code)?;
        println!("Tuned module saved as: {}", tuned_filename);
    } else {
        println!("Failed to extract modified code from the response.");
    }

    Ok(())
}
