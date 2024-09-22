mod cmd;
mod error;
mod toml;
mod config_man;
mod license;

use std::env;
use std::io::{self, Write};
use std::process::ExitCode;
use std::fs;

use clap::Parser;

use crate::cmd::{Cmd, Execute};
use crate::error::SilentExit;

use crate::config_man::{get_config_path, create_config, set_analytics};
use crate::license::check_license;

#[tokio::main]
pub async fn main() -> ExitCode {
    dotenv::dotenv().ok();
    // Forcibly disable backtraces.
    env::remove_var("RUST_LIB_BACKTRACE");
    env::remove_var("RUST_BACKTRACE");
    
    if let Err(e) = check_license().await {
        eprintln!("License check failed: {}", e);
        eprintln!("Check your license and try again. Contact team@getinstachip.com for assistance.");
        return ExitCode::FAILURE;
    }

    let flag_file = get_config_path().unwrap().with_file_name(".vpm_welcome_shown");
    if !flag_file.exists() {
        if let Err(e) = create_config() {
            eprintln!("Failed to create config: {}", e);
            return ExitCode::FAILURE;
        }

        println!("Welcome to Instachip Pro!");
        println!("We collect anonymous usage data to improve the tool.");
        println!("The following information will be collected:");
        println!(" - The version of vpm you are using");
        println!(" - Which commands you run and when (not including arguments, input, or output)");
        println!("No personal information will be collected.\n");

        let mut input = String::new();
        loop {
            print!("Would you like to enable analytics? (y/n): ");
            io::stdout().flush().unwrap();
            io::stdin().read_line(&mut input).unwrap();
            input = input.trim().to_lowercase();
            if input == "y" || input == "n" {
                let enable_analytics = input.trim().to_lowercase() != "n";
                if let Err(e) = set_analytics(enable_analytics) {
                    eprintln!("Failed to set analytics preference: {}", e);
                    return ExitCode::FAILURE;
                }
                if enable_analytics {
                    println!("Analytics enabled.");
                } else {
                    println!("Analytics disabled.");
                }
                break;
            }
            println!("Invalid input. Please enter 'y' or 'n'.");
            input.clear();
        }

        println!("You can change this setting at any time by running `vpm config --analytics <true/false>`.\n");
        fs::write(flag_file, "").unwrap();
    }

    match Cmd::parse().execute().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => match e.downcast::<SilentExit>() {
            Ok(SilentExit { code }) => code.into(),
            Err(e) => {
                _ = writeln!(io::stderr(), "vpm: {e:?}");
                ExitCode::FAILURE
            }
        },
    }
}
