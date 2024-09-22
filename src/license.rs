use anyhow::Result;
use machine_uid;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, ACCEPT, CONTENT_TYPE};
use serde_json::Value;
use std::fs;
use std::io::{stdout, Write};
use std::time::{SystemTime, UNIX_EPOCH};
use rpassword::read_password;

use crate::config_man::{create_config, get_config_path};

const API_URL: &str = "https://api.keygen.sh/v1";
const ACCOUNT_ID: Option<&str> = option_env!("KEYGEN_ACCOUNT_ID");
const TOKEN: Option<&str> = option_env!("KEYGEN_TOKEN");

pub async fn check_license() -> Result<()> {
    let config_path = get_config_path().ok_or_else(|| anyhow::anyhow!("Failed to get config path"))?;
    if !config_path.exists() {
        create_config()?;
    }
    let last_check_file = config_path.with_file_name(".last_check");

    let current_time = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let should_check = if last_check_file.exists() {
        let last_check = fs::read_to_string(&last_check_file)?.parse::<u64>()?;
        current_time - last_check >= 86400 // 24 hours in seconds
    } else {
        true
    };

    if should_check {
        let client = reqwest::Client::new();
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, HeaderValue::from_str(&format!("Bearer {}", TOKEN.unwrap()))?);
        headers.insert(ACCEPT, HeaderValue::from_static("application/vnd.api+json"));
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/vnd.api+json"));

        let license_key = get_license_key_input();

        match validate_license(&client, &headers, &license_key).await {
            Ok((validation_code, license_id)) => {
                match validation_code.as_str() {
                    "VALID" => {
                        fs::write(&last_check_file, current_time.to_string())?;
                        return Ok(());
                    },
                    "FINGERPRINT_SCOPE_REQUIRED" => {
                        println!("License requires activation. Attempting to activate...");
                        match activate_license(&client, &headers, &license_id).await {
                            Ok(()) => println!("License activated successfully."),
                            Err(e) => println!("Error activating license: {}", e),
                        }
                    },
                    "NO_MACHINE" | "NO_MACHINES" => {
                        println!("License is not activated on this machine. Attempting to activate...");
                        match activate_license(&client, &headers, &license_id).await {
                            Ok(()) => println!("License activated successfully."),
                            Err(e) => println!("Error activating license: {}", e),
                        }
                    },
                    "TOO_MANY_MACHINES" => {
                        println!("Too many machines. Attempting to deactivate...");
                        match deactivate_license(&client, &headers, &license_id).await {
                            Ok(()) => println!("License deactivated successfully. Please reactivate on this machine."),
                            Err(e) => println!("Error deactivating license: {}", e),
                        }
                    },
                    "NOT_FOUND" | "SUSPENDED" | "OVERDUE" | "EXPIRED" => {
                        println!("License is {}. Please contact support.", validation_code);
                        std::process::exit(1);
                    },
                    _ => println!("Unexpected validation result: {}", validation_code),
                }
            },
            Err(e) => println!("Error validating license: {}", e),
        }

        fs::write(&last_check_file, current_time.to_string())?;
    }

    Ok(())
}

async fn validate_license(
    client: &reqwest::Client,
    headers: &HeaderMap,
    license_key: &str,
) -> Result<(String, String), Box<dyn std::error::Error>> {
    let url = format!("{}/accounts/{}/licenses/actions/validate-key", API_URL, ACCOUNT_ID.unwrap());
    let machine_fingerprint = machine_uid::get()?;
    let body = serde_json::json!({
        "meta": {
            "key": license_key.trim_end(),
            "scope": {
                "fingerprint": machine_fingerprint
            }
        }
    });

    let response = client
        .post(&url)
        .headers(headers.clone())
        .json(&body)
        .send()
        .await?;

    let status = response.status();
    let json: Value = response.json().await?;
    // println!("Response: {}", serde_json::to_string_pretty(&json).unwrap());

    if status.is_success() {
        let validation_code = json["meta"]["code"].as_str().unwrap_or("UNKNOWN").to_string();
        let license_id = json["data"]["id"].as_str().unwrap_or("").to_string();
        Ok((validation_code, license_id))
    } else {
        Err(format!("API error: {}", status).into())
    }
}

async fn activate_license(
    client: &reqwest::Client,
    headers: &HeaderMap,
    license_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let url = format!("{}/accounts/{}/machines", API_URL, ACCOUNT_ID.unwrap());
    let body = serde_json::json!({
        "data": {
            "type": "machines",
            "attributes": {
                "fingerprint": machine_uid::get()?,
                "name": "My Machine"
            },
            "relationships": {
                "license": {
                    "data": {
                        "type": "licenses",
                        "id": license_id
                    }
                }
            }
        }
    });

    let response = client
        .post(&url)
        .headers(headers.clone())
        .json(&body)
        .send()
        .await?;
  
    let status = response.status();
    let text = response.text().await?;
    // println!("Response: {}", serde_json::to_string_pretty(&serde_json::from_str::<serde_json::Value>(&text).unwrap_or_default()).unwrap());

    if status.is_success() {
        Ok(())
    } else {
        let _error_body = text;
        Err(format!("API error: {}", status).into())
    }
}

async fn deactivate_license(
    client: &reqwest::Client,
    headers: &HeaderMap,
    license_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let machine_fingerprint = machine_uid::get()?;
    let url = format!("{}/accounts/{}/machines/{}", API_URL, ACCOUNT_ID.unwrap(), machine_fingerprint);

    let response = client
        .delete(&url)
        .headers(headers.clone())
        .bearer_auth(license_id)
        .send()
        .await?;

    let status = response.status();
    if status.is_success() {
        Ok(())
    } else {
        let _error_body = response.text().await?;
        Err(format!("API error: {}", status).into())
    }
}

fn get_license_key_input() -> String {
    let path = get_config_path().unwrap().with_file_name("license.key");
    if !path.exists() || fs::read_to_string(&path).unwrap().trim().is_empty() {
        print!("License not found. Enter your license key: ");
        stdout().flush().unwrap();
        let key = read_password().unwrap().trim().to_string();
        fs::write(path, &key).unwrap();
        key
    } else {
        fs::read_to_string(path).unwrap()
    }
}