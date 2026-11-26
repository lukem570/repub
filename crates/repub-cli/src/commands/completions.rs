//! The `completions` command - generate shell completions.

use std::io;

use anyhow::Result;
use clap::{Args, CommandFactory, ValueEnum};
use clap_complete::{generate, Shell};

use crate::cli::Cli;

/// Arguments for the `completions` command.
#[derive(Args, Debug)]
pub struct CompletionsArgs {
    /// Shell to generate completions for
    #[arg(value_enum)]
    pub shell: ShellArg,
}

/// Shell argument.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ShellArg {
    /// Bash shell
    Bash,
    /// Zsh shell
    Zsh,
    /// Fish shell
    Fish,
    /// PowerShell
    Powershell,
    /// Elvish shell
    Elvish,
}

impl From<ShellArg> for Shell {
    fn from(arg: ShellArg) -> Self {
        match arg {
            ShellArg::Bash => Shell::Bash,
            ShellArg::Zsh => Shell::Zsh,
            ShellArg::Fish => Shell::Fish,
            ShellArg::Powershell => Shell::PowerShell,
            ShellArg::Elvish => Shell::Elvish,
        }
    }
}

/// Run the `completions` command.
pub fn run(args: CompletionsArgs) -> Result<()> {
    let mut cmd = Cli::command();
    let shell: Shell = args.shell.into();

    generate(shell, &mut cmd, "repub-cli", &mut io::stdout());

    Ok(())
}
