mod challenge;
mod error;
mod hosts;

use clap::Parser;
use error::{AppError, Result};
use hosts::{block_website, get_hosts_path, unblock_website}; // Import necessary functions

/// Get Work Done (gwd) - A command line tool to block/unblock websites.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Parser, Debug)]
enum Commands {
    /// Blocks a website by adding entries to the hosts file.
    Block {
        /// The domain name to block (e.g., example.com). 'www.' is handled automatically.
        #[arg(required = true)]
        domain: String,
    },
    /// Unblocks a website after a typing challenge.
    Unblock {
        /// The domain name to unblock (e.g., example.com). 'www.' is handled automatically.
        #[arg(required = true)]
        domain: String,

        /// Number of random words required for the unblock challenge. Set to 0 to disable.
        #[arg(long, default_value_t = 5, value_parser = clap::value_parser!(u16).range(0..))]
        // Allow 0
        challenge_length: u16,
    },
}

fn check_permissions() -> Result<()> {
    let hosts_path = get_hosts_path()?; // Get the path once
    #[cfg(windows)]
    {
        // Basic check: Can we open the hosts file for writing?
        // A more robust check involves checking the user's token, but this is simpler.
        match std::fs::OpenOptions::new()
            .write(true)
            .open(&hosts_path) // Use the fetched path
        {
            Ok(_) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                Err(AppError::PermissionDenied(hosts_path)) // Pass the path
            }
            Err(e) => Err(AppError::Io(format!("Error checking permissions on {:?}: {}", hosts_path, e))), // Convert error to string
        }
    }
    #[cfg(unix)] // Add cfg attribute for Unix block
    {
        // Check if the effective user ID is root (0)
        if !nix::unistd::Uid::effective().is_root() {
            Err(AppError::PermissionDenied(hosts_path)) // Pass the path
        } else {
            Ok(())
        }
    }
    #[cfg(not(any(unix, windows)))] // Handle other OSes
    {
        // Assume permissions are okay on unknown platforms for now
        println!("Warning: Unknown platform, cannot reliably check permissions.");
        Ok(())
    }
}

fn run() -> Result<()> {
    // Check permissions *before* parsing args or reading files
    check_permissions()?;

    let args = Args::parse();

    match args.command {
        Commands::Block { domain } => {
            println!("Attempting to block '{}'...", domain);
            // Call the combined block_website function from hosts module
            block_website(&domain)?;
            // Success messages are now handled within block_website
        }
        Commands::Unblock {
            domain,
            challenge_length,
        } => {
            println!("Attempting to unblock '{}'...", domain);
            // Call the combined unblock_website function from hosts module
            // It handles the challenge internally now based on the count
            unblock_website(&domain, challenge_length as usize)?;
            // Success messages are now handled within unblock_website
        }
    }

    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
