//! Output formatting utilities.

use std::fmt::Display;

use console::style;
use serde::Serialize;
use tabled::{Table, Tabled};

use crate::cli::{GlobalOptions, OutputFormat};

/// Print a success message.
pub fn success(msg: impl Display, opts: &GlobalOptions) {
    if opts.quiet {
        return;
    }
    if opts.no_color {
        println!("✓ {msg}");
    } else {
        println!("{} {msg}", style("✓").green().bold());
    }
}

/// Print an info message.
pub fn info(msg: impl Display, opts: &GlobalOptions) {
    if opts.quiet {
        return;
    }
    if opts.no_color {
        println!("ℹ {msg}");
    } else {
        println!("{} {msg}", style("ℹ").blue());
    }
}

/// Print a warning message.
pub fn warn(msg: impl Display, opts: &GlobalOptions) {
    if opts.quiet {
        return;
    }
    if opts.no_color {
        eprintln!("⚠ {msg}");
    } else {
        eprintln!("{} {msg}", style("⚠").yellow());
    }
}

/// Print a verbose message (only if verbose is enabled).
pub fn verbose(msg: impl Display, opts: &GlobalOptions) {
    if !opts.verbose || opts.quiet {
        return;
    }
    if opts.no_color {
        println!("  {msg}");
    } else {
        println!("  {}", style(msg).dim());
    }
}

/// Print data as JSON or formatted text.
pub fn print_data<T: Serialize + Display>(data: &T, opts: &GlobalOptions) -> anyhow::Result<()> {
    match opts.output {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(data)?;
            println!("{json}");
        }
        OutputFormat::Text => {
            println!("{data}");
        }
    }
    Ok(())
}

/// Print a table of data.
pub fn print_table<T: Tabled>(rows: &[T], opts: &GlobalOptions) {
    if rows.is_empty() {
        return;
    }

    let table = Table::new(rows);
    
    if opts.no_color {
        println!("{table}");
    } else {
        // Could add styling here
        println!("{table}");
    }
}

/// A key-value pair for display.
#[derive(Debug, Clone, Tabled)]
pub struct KeyValue {
    /// The key/label
    #[tabled(rename = "Property")]
    pub key: String,
    /// The value
    #[tabled(rename = "Value")]
    pub value: String,
}

impl KeyValue {
    /// Create a new key-value pair.
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
        }
    }
}

/// Print key-value pairs.
pub fn print_key_values(pairs: &[KeyValue], opts: &GlobalOptions) {
    if opts.quiet {
        return;
    }

    match opts.output {
        OutputFormat::Json => {
            let map: std::collections::HashMap<_, _> = pairs
                .iter()
                .map(|kv| (kv.key.clone(), kv.value.clone()))
                .collect();
            if let Ok(json) = serde_json::to_string_pretty(&map) {
                println!("{json}");
            }
        }
        OutputFormat::Text => {
            let max_key_len = pairs.iter().map(|kv| kv.key.len()).max().unwrap_or(0);
            for kv in pairs {
                if opts.no_color {
                    println!("{:width$}  {}", kv.key, kv.value, width = max_key_len);
                } else {
                    println!(
                        "{:width$}  {}",
                        style(&kv.key).bold(),
                        kv.value,
                        width = max_key_len
                    );
                }
            }
        }
    }
}
