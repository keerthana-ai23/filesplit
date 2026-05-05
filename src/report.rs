use std::path::Path;

use colored::Colorize;
use serde::{Deserialize, Serialize};

use crate::error::{FileSplitError, Result};
use crate::format::format_bytes;

// ─── Data structures ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkInfo {
    pub part: usize,
    pub path: String,
    pub rows: u64,
    pub bytes: u64,
    pub bytes_human: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SplitSummary {
    pub input_file: String,
    pub input_size_bytes: u64,
    pub input_size_human: String,
    pub format: String,
    pub split_mode: String,
    pub output_dir: String,
    pub total_chunks: usize,
    pub total_rows_written: u64,
    pub elapsed_seconds: f64,
    pub chunks: Vec<ChunkInfo>,
}

// ─── Terminal summary ─────────────────────────────────────────────────────────

pub fn print_summary(s: &SplitSummary) {
    let divider = "─".repeat(56);

    println!("{}", divider.dimmed());
    println!("{}", "  Split Complete".green().bold());
    println!("{}", divider.dimmed());

    println!(
        "  {:<22} {}",
        "Input file:".bold(),
        s.input_file.yellow()
    );
    println!(
        "  {:<22} {}",
        "Input size:".bold(),
        s.input_size_human.cyan()
    );
    println!(
        "  {:<22} {}",
        "Format:".bold(),
        s.format.cyan()
    );
    println!(
        "  {:<22} {}",
        "Split mode:".bold(),
        s.split_mode.cyan()
    );
    println!(
        "  {:<22} {}",
        "Chunks created:".bold(),
        s.total_chunks.to_string().green()
    );
    println!(
        "  {:<22} {}",
        "Total rows written:".bold(),
        format_number(s.total_rows_written).green()
    );
    println!(
        "  {:<22} {:.2}s",
        "Elapsed:".bold(),
        s.elapsed_seconds
    );

    println!("{}", divider.dimmed());
    println!("{}", "  Chunk Breakdown".bold());
    println!("{}", divider.dimmed());

    // Show a condensed table
    println!(
        "  {:<8} {:<12} {:<12} {}",
        "Part".bold(),
        "Rows".bold(),
        "Size".bold(),
        "File".bold()
    );

    let show_all = s.chunks.len() <= 20;
    let chunks_to_show: Vec<_> = if show_all {
        s.chunks.iter().collect()
    } else {
        // Show first 5 and last 5
        s.chunks.iter().take(5)
            .chain(std::iter::once(&s.chunks[0]).take(0)) // placeholder
            .collect()
    };

    for chunk in &chunks_to_show {
        println!(
            "  {:<8} {:<12} {:<12} {}",
            format!("{:04}", chunk.part).cyan(),
            format_number(chunk.rows),
            chunk.bytes_human.yellow(),
            Path::new(&chunk.path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&chunk.path)
                .dimmed()
        );
    }

    if !show_all {
        let skipped = s.chunks.len() - 5;
        println!("  {} … ({} more chunks) …", "".dimmed(), skipped);
        for chunk in s.chunks.iter().rev().take(5).collect::<Vec<_>>().iter().rev() {
            println!(
                "  {:<8} {:<12} {:<12} {}",
                format!("{:04}", chunk.part).cyan(),
                format_number(chunk.rows),
                chunk.bytes_human.yellow(),
                Path::new(&chunk.path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(&chunk.path)
                    .dimmed()
            );
        }
    }

    println!("{}", divider.dimmed());

    // Throughput
    if s.elapsed_seconds > 0.0 {
        let mb_per_sec = (s.input_size_bytes as f64 / 1024.0 / 1024.0) / s.elapsed_seconds;
        println!(
            "  {:<22} {:.1} MB/s",
            "Throughput:".bold(),
            mb_per_sec
        );
    }

    println!("{}", divider.dimmed());
    println!(
        "  {} Output in: {}",
        "✓".green().bold(),
        s.output_dir.yellow()
    );
    println!();
}

// ─── JSON report ──────────────────────────────────────────────────────────────

pub fn write_json_report(summary: &SplitSummary, path: &Path) -> Result<()> {
    let json = serde_json::to_string_pretty(summary)
        .map_err(|e| FileSplitError::ReportWrite(e.to_string()))?;
    std::fs::write(path, json)
        .map_err(|e| FileSplitError::ReportWrite(e.to_string()))?;
    Ok(())
}

// ─── Util ─────────────────────────────────────────────────────────────────────

fn format_number(n: u64) -> String {
    // Insert commas every 3 digits
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}
