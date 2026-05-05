use clap::{Parser, ValueEnum};
use std::path::PathBuf;

/// FileSplit — Safe, streaming CSV and text file splitter.
///
/// Splits large CSV or text files by size or row count while
/// preserving row boundaries and handling quoted CSV values correctly.
#[derive(Parser, Debug)]
#[command(
    name = "filesplit",
    version = "1.0.0",
    author,
    about,
    long_about = None,
    after_help = "EXAMPLES:\n  filesplit -f large.csv --size 100MB\n  filesplit -f large.csv --rows 500000 --preserve-header\n  filesplit -f data.csv --size 1GB --format csv --delimiter , --output ./chunks\n  filesplit -f events.log --rows 100000 --format text --suffix .log"
)]
pub struct Cli {
    /// Input file to split
    #[arg(short = 'f', long = "file", value_name = "FILE")]
    pub file: PathBuf,

    /// Split by maximum file size (e.g. 100MB, 1GB, 500KB)
    #[arg(
        long = "size",
        value_name = "SIZE",
        conflicts_with = "rows",
        help = "Maximum size per split file (e.g. 100MB, 1GB, 500KB)"
    )]
    pub size: Option<String>,

    /// Split by maximum number of rows per file
    #[arg(
        long = "rows",
        value_name = "N",
        conflicts_with = "size",
        help = "Maximum rows per split file (data rows only, excludes header)"
    )]
    pub rows: Option<u64>,

    /// File format — determines parsing strategy
    #[arg(
        long = "format",
        value_name = "FORMAT",
        default_value = "auto",
        help = "File format: auto, csv, tsv, text"
    )]
    pub format: FileFormat,

    /// CSV field delimiter (only for csv/tsv formats)
    #[arg(
        long = "delimiter",
        short = 'd',
        value_name = "CHAR",
        default_value = ",",
        help = "Field delimiter character"
    )]
    pub delimiter: char,

    /// Preserve the header row in every split file
    #[arg(
        long = "preserve-header",
        default_value = "true",
        help = "Copy the header row into every split file (CSV mode only)"
    )]
    pub preserve_header: bool,

    /// Output directory for split files
    #[arg(
        long = "output",
        short = 'o',
        value_name = "DIR",
        help = "Output directory (defaults to same directory as input file)"
    )]
    pub output: Option<PathBuf>,

    /// Prefix for output file names
    #[arg(
        long = "prefix",
        value_name = "PREFIX",
        help = "Output filename prefix (defaults to input filename stem)"
    )]
    pub prefix: Option<String>,

    /// File extension/suffix for output files
    #[arg(
        long = "suffix",
        value_name = "EXT",
        help = "Output file extension (e.g. .csv, .log) — defaults to input file extension"
    )]
    pub suffix: Option<String>,

    /// Number of digits in the part number (zero-padded)
    #[arg(
        long = "digits",
        value_name = "N",
        default_value = "4",
        help = "Zero-padding width for part numbers (e.g. 4 → part_0001.csv)"
    )]
    pub digits: usize,

    /// Write a JSON summary report alongside the output files
    #[arg(
        long = "report",
        value_name = "FILE",
        help = "Write a JSON split summary report to this path"
    )]
    pub report: Option<PathBuf>,

    /// Suppress progress bar
    #[arg(long = "quiet", short = 'q', help = "Suppress progress output")]
    pub quiet: bool,

    /// Print verbose output including per-chunk details
    #[arg(long = "verbose", short = 'v', help = "Verbose output")]
    pub verbose: bool,
}

#[derive(ValueEnum, Debug, Clone, PartialEq)]
pub enum FileFormat {
    /// Automatically detect from file extension
    Auto,
    /// Comma-separated values (safe quoted field handling)
    Csv,
    /// Tab-separated values
    Tsv,
    /// Plain text (split on newlines only)
    Text,
}

impl Cli {
    /// Validate that exactly one split mode is specified.
    pub fn validate(&self) -> crate::error::Result<()> {
        if self.size.is_none() && self.rows.is_none() {
            return Err(crate::error::FileSplitError::MissingMode);
        }
        if !self.file.exists() {
            return Err(crate::error::FileSplitError::FileNotFound(
                self.file.display().to_string(),
            ));
        }
        Ok(())
    }
}
