mod crypto;
use clap::{Parser, Subcommand};
use rpassword::read_password;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs;
use std::io::{self, Read, Write};
use std::process::Command;
use tempfile::NamedTempFile;
use zeroize::{Zeroize, ZeroizeOnDrop};

// A single diary entry
#[derive(Serialize, Deserialize, Debug, Clone, Zeroize, ZeroizeOnDrop)]
pub struct DiaryEntry {
    pub title: String,
    pub body: String,
}

// contains all the diary entries in vector format
#[derive(Serialize, Deserialize, Debug, Default, Zeroize, ZeroizeOnDrop)]
pub struct DiaryVault {
    pub entries: Vec<DiaryEntry>,
}

impl DiaryVault {
    // vault into raw JSON bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>, Box<dyn Error>> {
        let bytes = serde_json::to_vec(self)?;
        Ok(bytes)
    }

    // raw JSON bytes back into DiaryEntry struct
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Box<dyn Error>> {
        let vault: DiaryVault = serde_json::from_slice(bytes)?;
        Ok(vault)
    }
}

// launching vim on /tmp
fn edit_text(initial_content: &str) -> Result<String, Box<dyn Error>> {
    let mut temp_file = NamedTempFile::new()?;

    if !initial_content.is_empty() {
        temp_file.write_all(initial_content.as_bytes())?;
        temp_file.flush()?;
    }

    let status = Command::new("vim")
        .arg(temp_file.path())
        .status()?;

    if !status.success() {
        return Err("Vim exited with error.".into());
    }

    let mut updated_content = String::new();
    let mut reloaded_file = fs::File::open(temp_file.path())?;
    reloaded_file.read_to_string(&mut updated_content)?;

    Ok(updated_content.trim().to_string())
}

#[derive(Parser, Debug)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Add {
        #[arg(short, long)]
        title: Option<String>,
    },
    List,
    View { id: usize },
    Edit { id: usize },
    Delete { id: usize },
    ChangePassword,
}

fn prompt_password() -> Result<String, Box<dyn Error>> {
    print!("Enter master password: ");
    io::stdout().flush()?;
    let password = read_password()?;
    if password.is_empty() {
        return Err("Password cannot be empty.".into());
    }
    Ok(password)
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    let master_password = prompt_password()?;

    // Load vault using the new crypto module
    let mut vault = crypto::load_vault(&master_password)?;

    match &cli.command {
        Commands::Add { title } => {
            let entry_title = match title {
                Some(t) => t.clone(),
                None => {
                    print!("Entry Title: ");
                    io::stdout().flush()?;
                    let mut input = String::new();
                    io::stdin().read_line(&mut input)?;
                    input.trim().to_string()
                }
            };
            println!("Add the body");
            let body = edit_text("")?;

            if body.is_empty() {
                println!("Empty body");
                return Ok(());
            }
            vault.entries.push(DiaryEntry {
                title: entry_title,
                body,
            });

            crypto::save_vault(&vault, &master_password)?;
            println!("Success");
        }

        Commands::List => {
            if vault.entries.is_empty() {
                println!("No entries");
            } else {
                println!("Stored Diary Entries:");
                for (idx, entry) in vault.entries.iter().enumerate() {
                    println!("[#{}] {}", idx + 1, entry.title);
                }
            }
        }

        Commands::View { id } => {
            let index = id.saturating_sub(1);
            if index >= vault.entries.len() {
                return Err(format!("Entry #{} does not exist.", id).into());
            }
            let entry = &vault.entries[index];
            println!("Entry #{}: {}", id, entry.title);
            println!("\n{}\n", entry.body);
        }

        Commands::Edit { id } => {
            let index = id.saturating_sub(1);
            if index >= vault.entries.len() {
                return Err(format!("Entry #{} does not exist.", id).into());
            }

            println!("Modifying #{}", id);
            let updated_body = edit_text(&vault.entries[index].body)?;

            vault.entries[index].body = updated_body;
            crypto::save_vault(&vault, &master_password)?;
            println!("Success");
        }

        Commands::Delete { id } => {
            let index = id.saturating_sub(1);
            if index >= vault.entries.len() {
                return Err(format!("Entry #{} does not exist.", id).into());
            }
            let _removed = vault.entries.remove(index);
            crypto::save_vault(&vault, &master_password)?;
            println!("Success");
        }

        Commands::ChangePassword => {
            print!("Change Password:");
            io::stdout().flush()?;
            let new_password = read_password()?;
            
            crypto::save_vault(&vault, &new_password)?;
            println!("Success! Master password updated.");

        }
    }
    Ok(())
}