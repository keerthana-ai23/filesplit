use std::fs::{self, File};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};

use crate::cli::{Cli, FileFormat};
use crate::error::{FileSplitError, Result};
use crate::format::{format_bytes, parse_size};
use crate::report::{ChunkInfo, SplitSummary};

// ─── Public entry point ───────────────────────────────────────────────────────

pub fn run(cli: Cli) -> Result<SplitSummary> {
    cli.validate()?;

    let format = resolve_format(&cli);
    let output_dir = resolve_output_dir(&cli)?;
    let prefix = resolve_prefix(&cli);
    let suffix = resolve_suffix(&cli, &format);

    println!(
        "{} {}",
        "→ Input:".bold(),
        cli.file.display().to_string().yellow()
    );
    println!("{} {}", "→ Format:".bold(), format_name(&format).cyan());
    println!("{} {}", "→ Output dir:".bold(), output_dir.display().to_string().yellow());

    let mode = resolve_mode(&cli)?;
    println!("{} {}", "→ Split mode:".bold(), mode.describe().green());
    println!();

    let input_size = cli.file.metadata()?.len();
    let start = Instant::now();

    let chunks = match format {
        FileFormat::Text => split_text(&cli, &mode, &output_dir, &prefix, &suffix)?,
        FileFormat::Csv | FileFormat::Tsv | FileFormat::Auto => {
            let delim = match format {
                FileFormat::Tsv => b'\t',
                _ => {
                    let d = cli.delimiter;
                    if d.is_ascii() {
                        d as u8
                    } else {
                        return Err(FileSplitError::InvalidDelimiter(d.to_string()));
                    }
                }
            };
            split_csv(&cli, &mode, &output_dir, &prefix, &suffix, delim)?
        }
    };

    let elapsed = start.elapsed();

    let summary = SplitSummary {
        input_file: cli.file.display().to_string(),
        input_size_bytes: input_size,
        input_size_human: format_bytes(input_size),
        format: format_name(&format).to_string(),
        split_mode: mode.describe(),
        output_dir: output_dir.display().to_string(),
        total_chunks: chunks.len(),
        total_rows_written: chunks.iter().map(|c| c.rows).sum(),
        elapsed_seconds: elapsed.as_secs_f64(),
        chunks: chunks.clone(),
    };

    // Optional JSON report
    if let Some(report_path) = &cli.report {
        crate::report::write_json_report(&summary, report_path)?;
        println!(
            "\n{} {}",
            "✓ Report written to:".green().bold(),
            report_path.display()
        );
    }

    Ok(summary)
}

// ─── Mode ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum SplitMode {
    BySize(u64),  // max bytes per chunk
    ByRows(u64),  // max rows per chunk
}

impl SplitMode {
    pub fn describe(&self) -> String {
        match self {
            SplitMode::BySize(b) => format!("size ≤ {}", format_bytes(*b)),
            SplitMode::ByRows(r) => format!("{} rows/chunk", r),
        }
    }
}

fn resolve_mode(cli: &Cli) -> Result<SplitMode> {
    if let Some(size_str) = &cli.size {
        let bytes = parse_size(size_str)?;
        Ok(SplitMode::BySize(bytes))
    } else if let Some(rows) = cli.rows {
        Ok(SplitMode::ByRows(rows))
    } else {
        Err(FileSplitError::MissingMode)
    }
}

// ─── CSV Splitting ────────────────────────────────────────────────────────────

fn split_csv(
    cli: &Cli,
    mode: &SplitMode,
    output_dir: &Path,
    prefix: &str,
    suffix: &str,
    delimiter: u8,
) -> Result<Vec<ChunkInfo>> {
    let input_size = cli.file.metadata()?.len();
    let pb = make_progress_bar(input_size, cli.quiet);

    let file = File::open(&cli.file)?;
    let buf_reader = BufReader::with_capacity(8 * 1024 * 1024, file); // 8 MB read buffer

    let mut reader = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .flexible(true)
        .has_headers(cli.preserve_header)
        .from_reader(buf_reader);

    // Capture the header row
    let header: Option<Vec<String>> = if cli.preserve_header {
        Some(
            reader
                .headers()
                .map_err(|e| FileSplitError::CsvParse { row: 0, source: e })?
                .iter()
                .map(|s| s.to_string())
                .collect(),
        )
    } else {
        None
    };

    let mut chunks: Vec<ChunkInfo> = Vec::new();
    let mut part = 1usize;
    let mut current_writer: Option<BufWriter<File>> = None;
    let mut current_rows: u64 = 0;
    let mut current_bytes: u64 = 0;
    let mut current_path = PathBuf::new();
    let mut global_row: u64 = 0;

    let mut open_chunk = |part: usize| -> Result<(BufWriter<File>, PathBuf)> {
        let path = chunk_path(output_dir, prefix, suffix, part, cli.digits);
        let f = File::create(&path)?;
        let mut w = BufWriter::with_capacity(8 * 1024 * 1024, f);
        // Write header if needed
        if let Some(h) = &header {
            let row = h.join(&(delimiter as char).to_string());
            writeln!(w, "{}", row)?;
        }
        Ok((w, path))
    };

    // Open the first chunk
    let (w, p) = open_chunk(part)?;
    current_writer = Some(w);
    current_path = p;

    let mut records = reader.records();
    loop {
        let record = match records.next() {
            None => break,
            Some(r) => r.map_err(|e| FileSplitError::CsvParse { row: global_row + 1, source: e })?,
        };

        global_row += 1;

        // Serialize record back to CSV bytes
        let mut buf: Vec<u8> = Vec::new();
        {
            let mut wtr = csv::WriterBuilder::new()
                .delimiter(delimiter)
                .has_headers(false)
                .from_writer(&mut buf);
            wtr.write_record(&record)
                .map_err(|e| FileSplitError::CsvParse { row: global_row, source: e })?;
            wtr.flush()?;
        }
        let row_len = buf.len() as u64;

        // Check if we need to roll to the next chunk
        let roll = match mode {
            SplitMode::BySize(max_bytes) => current_bytes + row_len > *max_bytes && current_rows > 0,
            SplitMode::ByRows(max_rows) => current_rows >= *max_rows,
        };

        if roll {
            // Flush and close current chunk
            if let Some(ref mut w) = current_writer {
                w.flush()?;
            }
            chunks.push(ChunkInfo {
                part,
                path: current_path.display().to_string(),
                rows: current_rows,
                bytes: current_bytes,
                bytes_human: format_bytes(current_bytes),
            });

            if cli.verbose {
                eprintln!(
                    "  {} Part {:0width$} → {} rows, {}",
                    "✓".green(),
                    part,
                    current_rows,
                    format_bytes(current_bytes),
                    width = cli.digits
                );
            }

            part += 1;
            current_rows = 0;
            current_bytes = 0;
            let (w, p) = open_chunk(part)?;
            current_writer = Some(w);
            current_path = p;
        }

        if let Some(ref mut w) = current_writer {
            w.write_all(&buf)?;
        }
        current_rows += 1;
        current_bytes += row_len;
        pb.inc(row_len);
    }

    // Finalize last chunk
    if current_rows > 0 {
        if let Some(ref mut w) = current_writer {
            w.flush()?;
        }
        chunks.push(ChunkInfo {
            part,
            path: current_path.display().to_string(),
            rows: current_rows,
            bytes: current_bytes,
            bytes_human: format_bytes(current_bytes),
        });
        if cli.verbose {
            eprintln!(
                "  {} Part {:0width$} → {} rows, {}",
                "✓".green(),
                part,
                current_rows,
                format_bytes(current_bytes),
                width = cli.digits
            );
        }
    } else {
        // Empty last chunk — remove the file
        let _ = fs::remove_file(&current_path);
    }

    pb.finish_and_clear();
    Ok(chunks)
}

// ─── Text Splitting ───────────────────────────────────────────────────────────

fn split_text(
    cli: &Cli,
    mode: &SplitMode,
    output_dir: &Path,
    prefix: &str,
    suffix: &str,
) -> Result<Vec<ChunkInfo>> {
    let input_size = cli.file.metadata()?.len();
    let pb = make_progress_bar(input_size, cli.quiet);

    let file = File::open(&cli.file)?;
    let reader = BufReader::with_capacity(8 * 1024 * 1024, file);

    let mut chunks: Vec<ChunkInfo> = Vec::new();
    let mut part = 1usize;
    let mut current_rows: u64 = 0;
    let mut current_bytes: u64 = 0;

    let first_path = chunk_path(output_dir, prefix, suffix, part, cli.digits);
    let mut writer = BufWriter::with_capacity(8 * 1024 * 1024, File::create(&first_path)?);
    let mut current_path = first_path;

    for line_result in reader.lines() {
        let line = line_result?;
        let line_bytes = (line.len() as u64) + 1; // +1 for the newline

        // Roll check (only after at least one line is written)
        let roll = match mode {
            SplitMode::BySize(max) => current_bytes + line_bytes > *max && current_rows > 0,
            SplitMode::ByRows(max) => current_rows >= *max,
        };

        if roll {
            writer.flush()?;
            chunks.push(ChunkInfo {
                part,
                path: current_path.display().to_string(),
                rows: current_rows,
                bytes: current_bytes,
                bytes_human: format_bytes(current_bytes),
            });

            if cli.verbose {
                eprintln!(
                    "  {} Part {:0width$} → {} lines, {}",
                    "✓".green(),
                    part,
                    current_rows,
                    format_bytes(current_bytes),
                    width = cli.digits
                );
            }

            part += 1;
            current_rows = 0;
            current_bytes = 0;
            current_path = chunk_path(output_dir, prefix, suffix, part, cli.digits);
            writer = BufWriter::with_capacity(8 * 1024 * 1024, File::create(&current_path)?);
        }

        writeln!(writer, "{}", line)?;
        current_rows += 1;
        current_bytes += line_bytes;
        pb.inc(line_bytes);
    }

    // Flush last chunk
    if current_rows > 0 {
        writer.flush()?;
        chunks.push(ChunkInfo {
            part,
            path: current_path.display().to_string(),
            rows: current_rows,
            bytes: current_bytes,
            bytes_human: format_bytes(current_bytes),
        });
    } else {
        let _ = fs::remove_file(&current_path);
    }

    pb.finish_and_clear();
    Ok(chunks)
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn chunk_path(dir: &Path, prefix: &str, suffix: &str, part: usize, digits: usize) -> PathBuf {
    dir.join(format!("{}_part_{:0width$}{}", prefix, part, suffix, width = digits))
}

fn resolve_format(cli: &Cli) -> FileFormat {
    match cli.format {
        FileFormat::Auto => {
            let ext = cli
                .file
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();
            match ext.as_str() {
                "tsv" => FileFormat::Tsv,
                "txt" | "log" | "ndjson" | "jsonl" => FileFormat::Text,
                _ => FileFormat::Csv, // default to CSV (handles .csv and anything unknown)
            }
        }
        ref f => f.clone(),
    }
}

fn resolve_output_dir(cli: &Cli) -> Result<PathBuf> {
    let dir = if let Some(ref out) = cli.output {
        out.clone()
    } else {
        cli.file
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf()
    };
    fs::create_dir_all(&dir)
        .map_err(|_| FileSplitError::OutputDirError(dir.display().to_string()))?;
    Ok(dir)
}

fn resolve_prefix(cli: &Cli) -> String {
    cli.prefix.clone().unwrap_or_else(|| {
        cli.file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("output")
            .to_string()
    })
}

fn resolve_suffix(cli: &Cli, format: &FileFormat) -> String {
    if let Some(ref s) = cli.suffix {
        return s.clone();
    }
    // Use the input file's extension
    let ext = cli
        .file
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or(match format {
            FileFormat::Tsv => "tsv",
            FileFormat::Text => "txt",
            _ => "csv",
        });
    format!(".{}", ext)
}

fn format_name(format: &FileFormat) -> &'static str {
    match format {
        FileFormat::Csv | FileFormat::Auto => "CSV",
        FileFormat::Tsv => "TSV",
        FileFormat::Text => "Text",
    }
}

fn make_progress_bar(total_bytes: u64, quiet: bool) -> ProgressBar {
    if quiet {
        return ProgressBar::hidden();
    }
    let pb = ProgressBar::new(total_bytes);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.cyan} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")
            .unwrap()
            .progress_chars("█▉▊▋▌▍▎▏  "),
    );
    pb
}
