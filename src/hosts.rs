use crate::challenge::run_challenge;
use crate::error::{AppError, Result};
use lazy_static::lazy_static;
use regex::Regex;
use std::fs::{self, File, OpenOptions}; // Added fs
use std::io::{self, BufRead, BufReader, Read, Seek, SeekFrom, Write}; // Added Read trait
use std::path::PathBuf; // Keep PathBuf

const REDIRECT_IP: &str = "0.0.0.0";
const BLOCK_COMMENT_TAG: &str = "# Blocked by gwd";

lazy_static! {
    // Regex to clean domain names (remove http/https, trailing slashes), case-insensitive protocol
    static ref DOMAIN_CLEANUP_REGEX: Result<Regex> = Regex::new(r"(?i)^(?:https?://)?(.*?)/?$").map_err(AppError::from);
    // Regex to find existing block entries more precisely
    // Matches start of line, optional whitespace, redirect IP, one or more spaces,
    // the domain, then either whitespace/comment or end of line.
    static ref HOSTS_ENTRY_REGEX: Result<Regex> = Regex::new(r"^\s*0\.0\.0\.0\s+").map_err(AppError::from); // Simplified for now, needs domain added dynamically

    // Determine hosts file path based on OS
    static ref HOSTS_PATH: Result<PathBuf> = get_hosts_path_internal();
}

// Internal function to determine the path, called by lazy_static
fn get_hosts_path_internal() -> Result<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        Ok(PathBuf::from(r"C:\Windows\System32\drivers\etc\hosts"))
    }
    #[cfg(any(
        target_os = "macos",
        target_os = "linux",
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "openbsd",
        target_os = "netbsd"
    ))]
    {
        Ok(PathBuf::from("/etc/hosts"))
    }
    #[cfg(not(any(
        target_os = "windows",
        target_os = "macos",
        target_os = "linux",
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "openbsd",
        target_os = "netbsd"
    )))]
    {
        Err(AppError::UnsupportedOS(std::env::consts::OS.to_string()))
    }
}

// Public function to get the cached hosts path
pub fn get_hosts_path() -> Result<PathBuf> {
    // Clone the Result itself. If it's Ok, the PathBuf inside is cloned.
    // If it's Err, the AppError inside is cloned (requires AppError to be Clone).
    HOSTS_PATH.clone()
}

// Function to format domain names consistently
fn format_domain_for_hosts(domain: &str) -> Result<String> {
    // Access the regex result, propagating errors using ?
    let regex = DOMAIN_CLEANUP_REGEX.as_ref().map_err(|e| e.clone())?;
    let cleaned = regex
        .captures(domain)
        .and_then(|cap| cap.get(1))
        .map(|m| m.as_str().to_lowercase())
        .ok_or_else(|| AppError::InvalidDomain(domain.to_string()))?;

    if cleaned.is_empty() {
        Err(AppError::InvalidDomain(domain.to_string()))
    } else {
        Ok(cleaned)
    }
}

// Function to add block entries to the hosts file
pub fn block_website(domain: &str) -> Result<()> {
    let hosts_path = get_hosts_path()?;
    let clean_domain = format_domain_for_hosts(domain)?;
    let domain_www = format!("www.{}", clean_domain);

    let entry1 = format!("{} {} {}", REDIRECT_IP, clean_domain, BLOCK_COMMENT_TAG);
    let entry2 = format!("{} {} {}", REDIRECT_IP, domain_www, BLOCK_COMMENT_TAG);

    // Regex for checking existing entries for *this specific domain*
    // Needs to be created dynamically inside the function.
    // We need to escape the domain string in case it contains regex metacharacters.
    let escaped_clean_domain = regex::escape(&clean_domain);
    let escaped_domain_www = regex::escape(&domain_www);
    let check_regex1_str = format!(
        r"^\s*{}\s+{}\s*(?:#.*)?$",
        regex::escape(REDIRECT_IP),
        escaped_clean_domain
    );
    let check_regex2_str = format!(
        r"^\s*{}\s+{}\s*(?:#.*)?$",
        regex::escape(REDIRECT_IP),
        escaped_domain_www
    );
    let check_regex1 = Regex::new(&check_regex1_str)?;
    let check_regex2 = Regex::new(&check_regex2_str)?;

    // Use a specific error mapping function
    let map_io_error = |e: io::Error, path: &PathBuf| match e.kind() {
        io::ErrorKind::PermissionDenied => AppError::PermissionDenied(path.clone()),
        // Use ReadHosts/WriteHosts specific variants if needed, otherwise generic Io
        _ => AppError::Io(format!("Failed access hosts file at {:?}: {}", path, e)),
    };

    let mut file = OpenOptions::new()
        .read(true)
        .append(true) // Use append mode for adding lines
        .open(&hosts_path)
        .map_err(|e| map_io_error(e, &hosts_path))?; // Use specific error mapping

    // Read existing content to check for duplicates
    // We need to rewind the file cursor because we opened in append mode initially
    file.seek(SeekFrom::Start(0))?;
    let reader = BufReader::new(&file);
    let mut lines_to_add = Vec::new();
    let mut exists1 = false;
    let mut exists2 = false;

    for line_result in reader.lines() {
        // Iterate over Result<String>
        let line = line_result?; // Handle potential IO error during read
        if check_regex1.is_match(&line) {
            exists1 = true;
        }
        if check_regex2.is_match(&line) {
            exists2 = true;
        }
        if exists1 && exists2 {
            break; // No need to read further
        }
    }

    if !exists1 {
        lines_to_add.push(entry1);
        println!("Adding entry for: {}", clean_domain);
    } else {
        println!("Block entry for {} already exists.", clean_domain);
    }

    if !exists2 {
        lines_to_add.push(entry2);
        println!("Adding entry for: {}", domain_www);
    } else {
        println!("Block entry for {} already exists.", domain_www);
    }

    if !lines_to_add.is_empty() {
        // Ensure the file ends with a newline before appending
        // Check the last byte of the file
        let mut last_char = [0; 1];
        // Check if file has content before seeking
        if file.metadata()?.len() > 0 {
            if file.seek(SeekFrom::End(-1)).is_ok() && file.read(&mut last_char).is_ok() {
                if last_char[0] != b'\n' {
                    // Removed unnecessary parentheses
                    file.seek(SeekFrom::End(0))?;
                    writeln!(file)?;
                }
            } else {
                // Error seeking or reading last byte, assume newline needed or proceed
                file.seek(SeekFrom::End(0))?;
            }
        } else {
            // File is empty, ensure cursor is at the start/end (which is the same)
            file.seek(SeekFrom::Start(0))?; // Or SeekFrom::End(0)
        }

        for line in lines_to_add {
            writeln!(file, "{}", line)?;
        }
        println!(
            "Successfully updated hosts file to block '{}'.",
            clean_domain
        );
        // Consider adding platform-specific flush DNS instructions here
        #[cfg(target_os = "windows")]
        println!("Run 'ipconfig /flushdns' if the block doesn't take effect immediately.");
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        println!("DNS cache might need flushing (e.g., systemd-resolve --flush-caches or dscacheutil -flushcache).");
    } else {
        println!("'{}' already configured for blocking.", clean_domain);
    }

    Ok(())
}

// Function to remove block entries from the hosts file
pub fn unblock_website(domain: &str, challenge_word_count: usize) -> Result<()> {
    let clean_domain = format_domain_for_hosts(domain)?;

    // Run the challenge first
    run_challenge(&clean_domain, challenge_word_count)?;

    let hosts_path = get_hosts_path()?;
    let domain_www = format!("www.{}", clean_domain);

    // Regex for finding the lines to remove
    // Needs to be created dynamically inside the function.
    let escaped_clean_domain = regex::escape(&clean_domain);
    let escaped_domain_www = regex::escape(&domain_www);
    // Match lines starting with the redirect IP, spaces, the domain (or www.domain), and then optional space/comment or end of line
    let remove_regex_str = format!(
        r"^\s*{}\s+({}|{})\s*(?:#.*)?$",
        regex::escape(REDIRECT_IP),
        escaped_clean_domain,
        escaped_domain_www
    );
    let remove_regex = Regex::new(&remove_regex_str)?;

    let temp_file_path = hosts_path.with_extension("tmp");

    // Use the same error mapping helper
    let map_io_error = |e: io::Error, path: &PathBuf| match e.kind() {
        io::ErrorKind::PermissionDenied => AppError::PermissionDenied(path.clone()),
        _ => AppError::Io(format!("Failed access hosts file at {:?}: {}", path, e)),
    };

    // Read from original, write non-matching lines to temp
    {
        // Scope for file handles
        let original_file = File::open(&hosts_path).map_err(|e| map_io_error(e, &hosts_path))?; // Use helper
        let reader = BufReader::new(original_file);

        let mut temp_file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&temp_file_path)
            .map_err(|e| AppError::Io(format!("Failed to create temp file: {}", e)))?;

        let mut removed_count = 0;
        for line_result in reader.lines() {
            // Iterate over Result<String>
            let line = line_result?; // Handle potential IO error during read
            if remove_regex.is_match(&line) {
                println!("Removing line: {}", line);
                removed_count += 1;
            } else {
                writeln!(temp_file, "{}", line)?;
            }
        }

        if removed_count == 0 {
            println!("No active blocking entries found for '{}'.", clean_domain);
            // Clean up temp file if nothing was removed
            drop(temp_file); // Close the file handle before removing
            fs::remove_file(&temp_file_path)?; // Use fs::remove_file
            return Ok(());
        }
    } // Files are closed here

    // Replace original with temp file
    fs::rename(&temp_file_path, &hosts_path).map_err(|e| {
        // Use fs::rename
        AppError::Io(format!(
            "Failed to replace hosts file with updated version: {}. Temp file at: {:?}",
            e, temp_file_path
        ))
    })?;

    println!(
        "Successfully removed blocking entries for '{}'.",
        clean_domain
    );
    // Consider adding platform-specific flush DNS instructions here
    #[cfg(target_os = "windows")]
    println!("Run 'ipconfig /flushdns' if you still cannot access the site.");
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    println!("DNS cache might need flushing (e.g., systemd-resolve --flush-caches or dscacheutil -flushcache).");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::NamedTempFile; // Now should resolve

    // Helper to create a mock hosts file
    fn create_mock_hosts(content: &str) -> io::Result<NamedTempFile> {
        let file = NamedTempFile::new()?;
        fs::write(file.path(), content)?;
        Ok(file)
    }

    // Helper function to override the HOSTS_PATH for testing
    #[allow(unused_variables)] // Silence warning for unused path
    fn set_test_hosts_path(path: PathBuf) {
        // This is tricky with lazy_static. A common approach is conditional compilation
        // or using a different mechanism for tests, like dependency injection.
        // For simplicity here, we'll rely on the functions accepting the path directly
        // if we refactor them, or acknowledge this limitation.
        // A more robust solution might involve an Arc<Mutex<Option<PathBuf>>>
        // or a testing-specific configuration setup.
        // Currently, the functions use the lazy_static directly, making this hard.
        // We will test the logic assuming the path resolution works.
        panic!("set_test_hosts_path is not implemented due to lazy_static limitations in test context. Refactor needed for full path mocking.");
    }

    #[test]
    fn test_format_domain_for_hosts_simple() {
        assert_eq!(
            format_domain_for_hosts("example.com").unwrap(),
            "example.com"
        );
    }

    #[test]
    fn test_format_domain_for_hosts_http() {
        assert_eq!(
            format_domain_for_hosts("http://example.com").unwrap(),
            "example.com"
        );
    }

    #[test]
    fn test_format_domain_for_hosts_https() {
        assert_eq!(
            format_domain_for_hosts("https://example.com").unwrap(),
            "example.com"
        );
    }

    #[test]
    fn test_format_domain_for_hosts_trailing_slash() {
        assert_eq!(
            format_domain_for_hosts("example.com/").unwrap(),
            "example.com"
        );
    }
    #[test]
    fn test_format_domain_for_hosts_https_trailing_slash() {
        assert_eq!(
            format_domain_for_hosts("https://example.com/").unwrap(),
            "example.com"
        );
    }

    #[test]
    fn test_format_domain_for_hosts_mixed_case() {
        assert_eq!(
            format_domain_for_hosts("HTTPS://Example.Com/").unwrap(),
            "example.com"
        );
    }

    #[test]
    fn test_format_domain_for_hosts_with_www() {
        // format_domain should NOT strip www
        assert_eq!(
            format_domain_for_hosts("www.example.com").unwrap(),
            "www.example.com"
        );
        assert_eq!(
            format_domain_for_hosts("http://www.example.com").unwrap(),
            "www.example.com"
        );
    }

    #[test]
    fn test_format_domain_for_hosts_invalid() {
        assert!(format_domain_for_hosts("").is_err());
        assert!(format_domain_for_hosts("http://").is_err());
        assert!(format_domain_for_hosts("https://").is_err());
        // Consider adding more invalid cases if needed
    }

    // --- Tests for block_website and unblock_website ---
    // These tests are more complex due to file I/O and lazy_static HOSTS_PATH.
    // A better approach would be to refactor block/unblock to accept a PathBuf
    // argument, allowing easy testing with temp files.

    // Example test structure (assuming refactor to pass path):
    /*
    fn block_website_testable(domain: &str, hosts_path: &PathBuf) -> Result<()> { ... }
    fn unblock_website_testable(domain: &str, challenge_word_count: usize, hosts_path: &PathBuf) -> Result<()> { ... }

    #[test]
    fn test_block_website_new_entry() {
        let mock_hosts = create_mock_hosts("127.0.0.1 localhost\n").unwrap();
        let hosts_path = mock_hosts.path().to_path_buf();

        block_website_testable("example.com", &hosts_path).unwrap();

        let content = fs::read_to_string(&hosts_path).unwrap();
        assert!(content.contains("0.0.0.0 example.com # Blocked by gwd"));
        assert!(content.contains("0.0.0.0 www.example.com # Blocked by gwd"));
    }

     #[test]
    fn test_unblock_website_removes_entries() {
        let initial_content = "127.0.0.1 localhost\n0.0.0.0 example.com # Blocked by gwd\n0.0.0.0 www.example.com # Blocked by gwd\n";
        let mock_hosts = create_mock_hosts(initial_content).unwrap();
        let hosts_path = mock_hosts.path().to_path_buf();

        // Mocking the challenge is needed here, or setting word_count to 0
        unblock_website_testable("example.com", 0, &hosts_path).unwrap(); // Skip challenge

        let content = fs::read_to_string(&hosts_path).unwrap();
        assert!(!content.contains("0.0.0.0 example.com"));
        assert!(!content.contains("0.0.0.0 www.example.com"));
        assert!(content.contains("127.0.0.1 localhost")); // Ensure other lines remain
    }
    */

    // Since refactoring is out of scope for this step, we acknowledge the limitation
    // that testing block/unblock directly against the real hosts file is risky
    // and mocking lazy_static is complex. Manual testing after build is needed.
}
