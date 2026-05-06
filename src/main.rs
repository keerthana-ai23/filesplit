mod config;
pub mod cli;
pub mod error;
pub mod splitter;
pub mod report;
pub mod format;

use anyhow::Result;
use cli::Cli;
use clap::Parser;
use colored::Colorize;

fn main() -> Result<()> {
    let cli = Cli::parse();

    print_banner();

    let result = splitter::run(cli);

    match result {
        Ok(summary) => {
            report::print_summary(&summary);
            Ok(())
        }
        Err(e) => {
            eprintln!("{} {}", "✗ Error:".red().bold(), e);
            std::process::exit(1);
        }
    }
}

fn print_banner() {
    println!("{}", "╔══════════════════════════════════════╗".cyan());
    println!("{}", "║     FileSplit — Safe File Splitter   ║".cyan());
    println!("{}", "╚══════════════════════════════════════╝".cyan());
    println!();
}
