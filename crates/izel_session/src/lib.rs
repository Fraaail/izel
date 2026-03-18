//! Compilation session state and configuration.

use clap::Parser;
use std::path::PathBuf;

/// The primary configuration for an Izel compilation session.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct SessionOptions {
    /// The input file to compile.
    pub input: Option<PathBuf>,

    /// The output path for the compiled binary.
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Emit specific IR or meta-information.
    #[arg(long)]
    pub emit: Option<String>,

    /// Optimize the output binary.
    #[arg(short = 'O', long, default_value = "0")]
    pub opt: String,

    /// Run the code after compilation using JIT.
    #[arg(long)]
    pub run: bool,

    /// Enable runtime contract checking (inject @requires/@ensures assertions).
    #[arg(long)]
    pub check_contracts: bool,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(clap::Subcommand, Debug, Clone)]
pub enum Command {
    /// Format a source file
    Fmt {
        /// The file to format
        input: PathBuf,
    },
    /// Start the language server
    Lsp,
    /// Resolve project dependencies
    Deps {
        /// Path to Izel.toml
        manifest_path: PathBuf,
    },
}

pub struct Session {
    pub options: SessionOptions,
}

impl Session {
    pub fn new(options: SessionOptions) -> Self {
        Self { options }
    }
}
