// src/bin/client.rs

use clap::{Parser, Subcommand};
use reqwest::Client;
use serde_json::Value;
use std::io::{self, Write};

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    /// Key-value server URL (e.g., http://localhost:3000)
    #[clap(short, long, default_value = "http://localhost:3000")]
    server: String,

    #[clap(subcommand)]
    command: Option<Commands>,

    /// Run in REPL mode if no command is given
    #[clap(long, default_value_t = false)]
    repl: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Set a new key (fails if key exists)
    Set {
        key: String,
        #[clap(value_parser = parse_json)]
        value: Value,
    },
    /// Update an existing key (fails if key doesn't exist)
    Update {
        key: String,
        #[clap(value_parser = parse_json)]
        value: Value,
    },
    /// Get a key's value
    Get {
        key: String,
    },
    /// Delete a key
    Delete {
        key: String,
    },
}

fn parse_json(s: &str) -> Result<Value, serde_json::Error> {
    serde_json::from_str(s)
}

async fn do_set(client: &Client, base_url: &str, key: &str, value: &Value) -> anyhow::Result<()> {
    let url = format!("{}/keys/{}", base_url.trim_end_matches('/'), urlencoding::encode(key));
    let res = client.post(&url).json(value).send().await?;
    let status = res.status();
    let body: serde_json::Value = res.json().await?;

    match status {
        reqwest::StatusCode::CREATED => {
            println!("OK: {}", body["uri"].as_str().unwrap_or("/unknown"));
        }
        reqwest::StatusCode::CONFLICT => {
            eprintln!("Error: key already exists. Use 'update' to modify.");
            std::process::exit(1);
        }
        _ => {
            eprintln!("Error: {} {:?}", status, body);
            std::process::exit(1);
        }
    }
    Ok(())
}

async fn do_update(client: &Client, base_url: &str, key: &str, value: &Value) -> anyhow::Result<()> {
    let url = format!("{}/keys/{}", base_url.trim_end_matches('/'), urlencoding::encode(key));
    let res = client.put(&url).json(value).send().await?;
    let status = res.status();
    let body: serde_json::Value = res.json().await?;

    match status {
        reqwest::StatusCode::OK => {
            println!("OK: {}", body["uri"].as_str().unwrap_or("/unknown"));
        }
        reqwest::StatusCode::NOT_FOUND => {
            eprintln!("Error: key does not exist. Use 'set' to create.");
            std::process::exit(1);
        }
        _ => {
            eprintln!("Error: {} {:?}", status, body);
            std::process::exit(1);
        }
    }
    Ok(())
}

async fn do_get(client: &Client, base_url: &str, key: &str) -> anyhow::Result<()> {
    let url = format!("{}/keys/{}", base_url.trim_end_matches('/'), urlencoding::encode(key));
    let res = client.get(&url).send().await?;
    let status = res.status();

    if status == reqwest::StatusCode::NOT_FOUND {
        eprintln!("Error: key not found");
        std::process::exit(1);
    }

    let body: serde_json::Value = res.json().await?;
    println!("{}", serde_json::to_string_pretty(&body)?);
    Ok(())
}

async fn do_delete(client: &Client, base_url: &str, key: &str) -> anyhow::Result<()> {
    let url = format!("{}/keys/{}", base_url.trim_end_matches('/'), urlencoding::encode(key));
    let res = client.delete(&url).send().await?;
    let status = res.status();

    if status == reqwest::StatusCode::NOT_FOUND {
        eprintln!("Error: key not found");
        std::process::exit(1);
    }

    let body: serde_json::Value = res.json().await?;
    println!("OK: {}", body["uri"].as_str().unwrap_or("/unknown"));
    Ok(())
}

async fn run_repl(server: String) -> anyhow::Result<()> {
    let client = Client::new();
    let mut rl = rustyline::DefaultEditor::new()?;
    println!("kvs-client REPL (server: {})", server);
    println!("Commands: set <key> <json>, update <key> <json>, get <key>, delete <key>, exit");
    loop {
        let readline = rl.readline("> ");
        match readline {
            Ok(line) => {
                rl.add_history_entry(line.as_str())?;
                let parts: Vec<&str> = line.trim().split_whitespace().collect();
                if parts.is_empty() {
                    continue;
                }

                match parts[0] {
                    "exit" | "quit" => break,
                    "set" if parts.len() >= 3 => {
                        let key = parts[1];
                        let json_str = parts[2..].join(" ");
                        match serde_json::from_str::<Value>(&json_str) {
                            Ok(value) => {
                                if let Err(e) = do_set(&client, &server, key, &value).await {
                                    eprintln!("Error: {}", e);
                                }
                            }
                            Err(e) => eprintln!("Invalid JSON: {}", e),
                        }
                    }
                    "update" if parts.len() >= 3 => {
                        let key = parts[1];
                        let json_str = parts[2..].join(" ");
                        match serde_json::from_str::<Value>(&json_str) {
                            Ok(value) => {
                                if let Err(e) = do_update(&client, &server, key, &value).await {
                                    eprintln!("Error: {}", e);
                                }
                            }
                            Err(e) => eprintln!("Invalid JSON: {}", e),
                        }
                    }
                    "get" if parts.len() == 2 => {
                        let key = parts[1];
                        let _ = do_get(&client, &server, key).await;
                    }
                    "delete" if parts.len() == 2 => {
                        let key = parts[1];
                        let _ = do_delete(&client, &server, key).await;
                    }
                    _ => eprintln!("Unknown command. Use: set|update|get|delete|exit"),
                }
            }
            Err(rustyline::error::ReadlineError::Interrupted) |
            Err(rustyline::error::ReadlineError::Eof) => {
                break;
            }
            Err(err) => {
                eprintln!("REPL error: {}", err);
                break;
            }
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    if cli.command.is_none() && !cli.repl {
        // Default to REPL if no command
        run_repl(cli.server).await?;
        return Ok(());
    }

    if let Some(cmd) = cli.command {
        let client = Client::new();
        match cmd {
            Commands::Set { key, value } => {
                do_set(&client, &cli.server, &key, &value).await?;
            }
            Commands::Update { key, value } => {
                do_update(&client, &cli.server, &key, &value).await?;
            }
            Commands::Get { key } => {
                do_get(&client, &cli.server, &key).await?;
            }
            Commands::Delete { key } => {
                do_delete(&client, &cli.server, &key).await?;
            }
        }
    } else if cli.repl {
        run_repl(cli.server).await?;
    }

    Ok(())
}