//! # Repub CLI
//!
//! Command-line interface for Repub desktop publishing.
//!
//! ## Commands
//!
//! - `new` - Create a new document
//! - `info` - Display document information
//! - `export` - Export to PDF, PNG, SVG, etc.
//! - `convert` - Convert between formats
//! - `add` - Add pages, shapes, text, images
//! - `list` - List document contents
//! - `validate` - Validate document structure
//! - `completions` - Generate shell completions
//!
//! ## Examples
//!
//! ```bash
//! # Create a new document
//! repub new --size letter output.pcd
//!
//! # Export to PDF
//! repub export document.pcd output.pdf
//!
//! # Get document info as JSON
//! repub info --json document.pcd
//! ```

#![warn(missing_docs)]

pub mod cli;
pub mod commands;
pub mod error;
pub mod output;

pub use cli::{Cli, Commands};
pub use error::{CliError, CliResult};

use anyhow::Result;

/// Run the CLI with parsed arguments.
pub fn run(cli: Cli) -> Result<()> {
    use commands::*;

    match cli.command {
        Commands::New(args) => new::run(args, &cli.global),
        Commands::Info(args) => info::run(args, &cli.global),
        Commands::Export(args) => export::run(args, &cli.global),
        Commands::Convert(args) => convert::run(args, &cli.global),
        Commands::Add(cmd) => add::run(cmd, &cli.global),
        Commands::List(args) => list::run(args, &cli.global),
        Commands::Validate(args) => validate::run(args, &cli.global),
        Commands::Template(cmd) => template::run(cmd, &cli.global),
        Commands::Completions(args) => completions::run(args),
    }
}
