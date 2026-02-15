//! Compare an SLX file with its round-tripped version.
//!
//! Usage:
//!
//! ```sh
//! cargo run --example roundtrip_compare -- path/to/model.slx
//! ```
//!
//! The tool reads the SLX file, writes it to a temporary file via the
//! generator, and then compares every ZIP entry byte-by-byte with the
//! original. It reports **all** differences.

use anyhow::{Context, Result};
use rustylink::model::SlxArchive;
use std::collections::BTreeMap;
use std::io::{Read, Seek};

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: roundtrip_compare <file.slx> [file2.slx ...]");
        std::process::exit(1);
    }

    let mut total_files = 0;
    let mut pass_count = 0;
    let mut fail_count = 0;

    for path in &args[1..] {
        total_files += 1;
        print!("Checking {} ... ", path);
        match compare_roundtrip(path) {
            Ok(diffs) => {
                if diffs.is_empty() {
                    println!("OK (exact match)");
                    pass_count += 1;
                } else {
                    println!("FAILED ({} differences)", diffs.len());
                    fail_count += 1;
                    for diff in &diffs {
                        println!("  {}", diff);
                    }
                }
            }
            Err(e) => {
                println!("ERROR: {}", e);
                fail_count += 1;
            }
        }
    }

    println!(
        "\n{}/{} files passed, {} failed",
        pass_count, total_files, fail_count
    );
    if fail_count > 0 {
        std::process::exit(1);
    }
    Ok(())
}

/// Compare an SLX file with its round-tripped version.
/// Returns a list of difference descriptions, empty if everything matches.
fn compare_roundtrip(path: &str) -> Result<Vec<String>> {
    // Read the original
    let archive = SlxArchive::from_file(path)
        .with_context(|| format!("Failed to read {}", path))?;

    // Write to a memory buffer
    let mut buf = std::io::Cursor::new(Vec::new());
    archive
        .write_to(&mut buf)
        .with_context(|| format!("Failed to write round-tripped {}", path))?;
    buf.set_position(0);

    // Read both ZIPs and compare entry-by-entry
    let original = read_zip_entries_from_file(path)?;
    let regenerated = read_zip_entries_from_reader(buf)?;

    let mut diffs = Vec::new();

    // Check for missing or extra entries
    let orig_keys: Vec<&String> = original.keys().collect();
    let regen_keys: Vec<&String> = regenerated.keys().collect();

    for key in &orig_keys {
        if !regenerated.contains_key(*key) {
            diffs.push(format!("MISSING entry: {}", key));
        }
    }
    for key in &regen_keys {
        if !original.contains_key(*key) {
            diffs.push(format!("EXTRA entry: {}", key));
        }
    }

    // Check entry order
    if orig_keys != regen_keys {
        diffs.push(format!(
            "Entry order differs:\n    Original:    {:?}\n    Regenerated: {:?}",
            orig_keys, regen_keys
        ));
    }

    // Compare content of each shared entry
    for (name, orig_data) in &original {
        if let Some(regen_data) = regenerated.get(name) {
            if orig_data != regen_data {
                // Try to show a useful diff
                let orig_str = String::from_utf8(orig_data.clone());
                let regen_str = String::from_utf8(regen_data.clone());
                match (orig_str, regen_str) {
                    (Ok(orig_text), Ok(regen_text)) => {
                        // Text comparison: show line-by-line diff
                        let orig_lines: Vec<&str> = orig_text.lines().collect();
                        let regen_lines: Vec<&str> = regen_text.lines().collect();
                        let mut line_diffs = Vec::new();
                        let max_lines = orig_lines.len().max(regen_lines.len());
                        for i in 0..max_lines {
                            let orig_line = orig_lines.get(i).copied().unwrap_or("<EOF>");
                            let regen_line = regen_lines.get(i).copied().unwrap_or("<EOF>");
                            if orig_line != regen_line {
                                line_diffs.push(format!(
                                    "    Line {}: \n      orig:  {}\n      regen: {}",
                                    i + 1,
                                    orig_line,
                                    regen_line
                                ));
                                if line_diffs.len() >= 10 {
                                    line_diffs.push(format!(
                                        "    ... and more (orig {} lines, regen {} lines)",
                                        orig_lines.len(),
                                        regen_lines.len()
                                    ));
                                    break;
                                }
                            }
                        }
                        diffs.push(format!(
                            "CONTENT DIFFERS: {} ({} bytes orig, {} bytes regen)\n{}",
                            name,
                            orig_data.len(),
                            regen_data.len(),
                            line_diffs.join("\n")
                        ));
                    }
                    _ => {
                        // Binary comparison
                        diffs.push(format!(
                            "CONTENT DIFFERS (binary): {} ({} bytes orig, {} bytes regen)",
                            name,
                            orig_data.len(),
                            regen_data.len()
                        ));
                    }
                }
            }
        }
    }

    Ok(diffs)
}

/// Read all ZIP entries from a file path, returning (name → bytes).
fn read_zip_entries_from_file(path: &str) -> Result<BTreeMap<String, Vec<u8>>> {
    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);
    read_zip_entries_from_reader(reader)
}

/// Read all ZIP entries from a reader, returning name → bytes in insertion order.
fn read_zip_entries_from_reader<R: Read + Seek>(reader: R) -> Result<BTreeMap<String, Vec<u8>>> {
    let mut zip = zip::ZipArchive::new(reader)?;
    let mut entries = BTreeMap::new();
    for i in 0..zip.len() {
        let mut file = zip.by_index(i)?;
        let name = file.name().to_string();
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;
        entries.insert(name, data);
    }
    Ok(entries)
}
