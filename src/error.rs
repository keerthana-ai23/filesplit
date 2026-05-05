use thiserror::Error;

pub type Result<T> = std::result::Result<T, FileSplitError>;

#[derive(Error, Debug)]
pub enum FileSplitError {
    #[error("You must specify either --size or --rows")]
    MissingMode,

    #[error("Input file not found: {0}")]
    FileNotFound(String),

    #[error("Cannot parse size string '{0}': use formats like 100MB, 1GB, 500KB")]
    InvalidSize(String),

    #[error("Invalid delimiter: '{0}'")]
    InvalidDelimiter(String),

    #[error("Output directory does not exist and could not be created: {0}")]
    OutputDirError(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("CSV parse error at row {row}: {source}")]
    CsvParse {
        row: u64,
        #[source]
        source: csv::Error,
    },

    #[error("Failed to write report: {0}")]
    ReportWrite(String),
}
