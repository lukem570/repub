//! CLI argument parsing with clap.

use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

use crate::commands::*;

/// Repub - Desktop publishing on the command line
#[derive(Parser, Debug)]
#[command(name = "repub-cli")]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    /// Subcommand to run
    #[command(subcommand)]
    pub command: Commands,

    /// Global options
    #[command(flatten)]
    pub global: GlobalOptions,
}

/// Global options available to all commands.
#[derive(Parser, Debug, Clone)]
pub struct GlobalOptions {
    /// Output format
    // Distinct id: subcommands have a positional named `output` (the file path),
    // and a shared id makes clap panic on downcast.
    #[arg(id = "output_format", long = "output", global = true, default_value = "text")]
    pub output: OutputFormat,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Suppress non-error output
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Configuration file path
    #[arg(long, global = true, env = "REPUB_CONFIG")]
    pub config: Option<PathBuf>,

    /// Disable colored output
    #[arg(long, global = true, env = "NO_COLOR")]
    pub no_color: bool,
}

/// Output format for command results.
#[derive(Debug, Clone, Copy, Default, ValueEnum, PartialEq, Eq)]
pub enum OutputFormat {
    /// Human-readable text output
    #[default]
    Text,
    /// JSON output for scripting
    Json,
}

/// Available subcommands.
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Create a new document
    New(new::NewArgs),

    /// Display document information
    Info(info::InfoArgs),

    /// Export document to another format
    Export(export::ExportArgs),

    /// Convert between document formats
    Convert(convert::ConvertArgs),

    /// Add content to a document
    #[command(subcommand)]
    Add(add::AddCommand),

    /// List document contents
    List(list::ListArgs),

    /// Validate document structure
    Validate(validate::ValidateArgs),

    /// Work with templates
    #[command(subcommand)]
    Template(template::TemplateCommand),

    /// Generate shell completions
    Completions(completions::CompletionsArgs),
}
