//! `genoforge` command-line interface.
//!
//! Every subcommand prints JSON to stdout so output can be piped straight into
//! the Python pipeline or `jq`. Diagnostics go to stderr.

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "genoforge", version, about = "Rust + Python genomics toolkit")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Print the core library version as JSON.
    Version,
    /// GC fraction of a raw sequence passed on the command line (smoke test).
    Gc {
        /// Nucleotide sequence, e.g. ACGTN.
        sequence: String,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let out = match cli.command {
        Command::Version => {
            serde_json::json!({ "core": genoforge_core::version(), "cli": env!("CARGO_PKG_VERSION") })
        }
        Command::Gc { sequence } => {
            serde_json::json!({ "gc_fraction": genoforge_core::gc_fraction(sequence.as_bytes()) })
        }
    };
    println!("{}", serde_json::to_string(&out)?);
    Ok(())
}
