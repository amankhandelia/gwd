use std::io;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug, Clone)] // Added Clone
pub enum AppError {
    #[error("I/O Error: {0}")]
    Io(String), // Store String to make it Cloneable

    // Removed 'source:' field name as String doesn't implement Error
    #[error("Failed to read hosts file at '{path}': {source_str}")]
    ReadHosts { path: PathBuf, source_str: String },

    #[error("Failed to write hosts file at '{path}': {source_str}")]
    WriteHosts { path: PathBuf, source_str: String },

    #[error("Invalid domain name: {0}")]
    InvalidDomain(String),

    #[error("Challenge failed: Incorrect sequence entered.")]
    ChallengeFailed,

    #[error(
        "Permission denied accessing hosts file at '{0}'. This application requires root/administrator privileges."
    )]
    PermissionDenied(PathBuf), // Include path for context

    #[error("Could not determine hosts file path for this operating system: {0}")]
    UnsupportedOS(String), // Renamed from UnknownHostsPath and added OS string

    #[error("Regex error: {0}")]
    Regex(#[from] regex::Error), // Use #[from] for regex::Error

    #[error("UTF-8 conversion error: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),
    // Removed the manual RegexError variant as #[from] handles it above
    // Removed Unknown error as it was too generic
}

// Implement From<io::Error> for AppError to simplify error handling
// This is still useful for generic IO errors outside file operations
impl From<io::Error> for AppError {
    fn from(err: io::Error) -> Self {
        AppError::Io(err.to_string()) // Convert io::Error to String
    }
}

// Removed manual From<regex::Error> implementation

pub type Result<T> = std::result::Result<T, AppError>;
