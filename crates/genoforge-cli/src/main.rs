//! `genoforge` command-line interface.
//!
//! Every subcommand prints JSON to stdout so output can be piped straight into
//! the Python pipeline or `jq`. Diagnostics go to stderr.

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use genoforge_core::fastq::{self, StatsOptions};

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
    /// Streaming QC statistics for a FASTQ file (plain or gzip, sniffed by content).
    FastqStats {
        /// Path to a .fastq or .fastq.gz file.
        path: PathBuf,
        /// Worker threads (0 = one per core).
        #[arg(long, default_value_t = 0)]
        threads: usize,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let out = match cli.command {
        Command::Version => {
            serde_json::json!({ "core": genoforge_core::version(), "cli": env!("CARGO_PKG_VERSION") })
        }
        Command::FastqStats { path, threads } => {
            let opts = StatsOptions {
                threads,
                ..StatsOptions::default()
            };
            serde_json::to_value(fastq::stats_from_path(&path, opts)?)?
        }
    };
    println!("{}", serde_json::to_string(&out)?);
    Ok(())
}
