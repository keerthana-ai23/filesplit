/// Integration tests for filesplit
///
/// These tests generate real files, run the splitter, then verify
/// all output files are valid and no data is lost.

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn temp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "filesplit_test_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .subsec_nanos()
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// Write a CSV with `n` data rows (plus a header) to a temp file.
/// Returns the path.
fn write_csv(dir: &PathBuf, name: &str, rows: usize) -> PathBuf {
    let path = dir.join(name);
    let mut f = fs::File::create(&path).unwrap();
    writeln!(f, "id,name,value,notes").unwrap();
    for i in 0..rows {
        // Include a row with a quoted field containing a comma
        if i % 10 == 0 {
            writeln!(f, "{},\"Smith, John\",{},\"note with, comma\"", i, i * 2).unwrap();
        } else {
            writeln!(f, "{},user_{},{},regular note", i, i, i * 2).unwrap();
        }
    }
    path
}

/// Write a plain text file with `n` lines.
fn write_text(dir: &PathBuf, name: &str, lines: usize) -> PathBuf {
    let path = dir.join(name);
    let mut f = fs::File::create(&path).unwrap();
    for i in 0..lines {
        writeln!(f, "Line number {} with some padding content here.", i).unwrap();
    }
    path
}

/// Count data rows (non-header) across all split files.
fn count_csv_data_rows(paths: &[PathBuf]) -> usize {
    let mut total = 0;
    for path in paths {
        let f = fs::File::open(path).unwrap();
        let reader = BufReader::new(f);
        // skip header
        total += reader.lines().skip(1).count();
    }
    total
}

fn count_text_lines(paths: &[PathBuf]) -> usize {
    let mut total = 0;
    for path in paths {
        let f = fs::File::open(path).unwrap();
        total += BufReader::new(f).lines().count();
    }
    total
}

/// Gather all split files matching prefix in dir.
fn gather_outputs(dir: &PathBuf, prefix: &str, ext: &str) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            name.starts_with(prefix) && name.ends_with(ext)
        })
        .collect();
    files.sort();
    files
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[test]
fn test_csv_split_by_rows_preserves_all_data() {
    let dir = temp_dir();
    let input = write_csv(&dir, "data.csv", 1000);
    let out_dir = dir.join("out_rows");
    fs::create_dir_all(&out_dir).unwrap();

    let cli = filesplit::cli::Cli {
        file: input,
        size: None,
        rows: Some(300),
        format: filesplit::cli::FileFormat::Csv,
        delimiter: ',',
        preserve_header: true,
        output: Some(out_dir.clone()),
        prefix: None,
        suffix: None,
        digits: 4,
        report: None,
        quiet: true,
        verbose: false,
    };

    let summary = filesplit::splitter::run(cli).unwrap();

    // Should have 4 chunks: 300 + 300 + 300 + 100
    assert_eq!(summary.total_chunks, 4);
    assert_eq!(summary.total_rows_written, 1000);

    let outputs = gather_outputs(&out_dir, "data", ".csv");
    assert_eq!(outputs.len(), 4);

    // Verify all rows preserved (no data loss)
    let total_data_rows = count_csv_data_rows(&outputs);
    assert_eq!(total_data_rows, 1000);

    // Verify every file has a header
    for path in &outputs {
        let f = fs::File::open(path).unwrap();
        let first_line = BufReader::new(f).lines().next().unwrap().unwrap();
        assert_eq!(first_line, "id,name,value,notes");
    }

    // Verify no file has more than 300 data rows (plus 1 header)
    for path in &outputs {
        let f = fs::File::open(path).unwrap();
        let lines: Vec<_> = BufReader::new(f).lines().collect();
        // data rows = total lines - 1 header
        assert!(lines.len() - 1 <= 300, "chunk exceeds row limit");
    }

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_csv_split_by_size_no_partial_rows() {
    let dir = temp_dir();
    let input = write_csv(&dir, "sized.csv", 2000);
    let out_dir = dir.join("out_size");
    fs::create_dir_all(&out_dir).unwrap();

    let cli = filesplit::cli::Cli {
        file: input,
        size: Some("5KB".to_string()),
        rows: None,
        format: filesplit::cli::FileFormat::Csv,
        delimiter: ',',
        preserve_header: true,
        output: Some(out_dir.clone()),
        prefix: None,
        suffix: None,
        digits: 4,
        report: None,
        quiet: true,
        verbose: false,
    };

    let summary = filesplit::splitter::run(cli).unwrap();

    assert!(summary.total_chunks > 1, "Expected multiple chunks");
    assert_eq!(summary.total_rows_written, 2000);

    // Verify all rows preserved
    let outputs = gather_outputs(&out_dir, "sized", ".csv");
    let total_data_rows = count_csv_data_rows(&outputs);
    assert_eq!(total_data_rows, 2000);

    // Verify every file is parseable as CSV
    for path in &outputs {
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_path(path)
            .unwrap();
        for result in rdr.records() {
            result.expect("CSV record should be valid");
        }
    }

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_text_split_by_rows_no_loss() {
    let dir = temp_dir();
    let input = write_text(&dir, "events.log", 500);
    let out_dir = dir.join("out_text");
    fs::create_dir_all(&out_dir).unwrap();

    let cli = filesplit::cli::Cli {
        file: input,
        size: None,
        rows: Some(100),
        format: filesplit::cli::FileFormat::Text,
        delimiter: ',',
        preserve_header: false,
        output: Some(out_dir.clone()),
        prefix: None,
        suffix: None,
        digits: 4,
        report: None,
        quiet: true,
        verbose: false,
    };

    let summary = filesplit::splitter::run(cli).unwrap();

    assert_eq!(summary.total_chunks, 5);
    assert_eq!(summary.total_rows_written, 500);

    let outputs = gather_outputs(&out_dir, "events", ".log");
    assert_eq!(outputs.len(), 5);

    let total_lines = count_text_lines(&outputs);
    assert_eq!(total_lines, 500);

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_quoted_csv_fields_not_split() {
    // Test that rows with quoted commas are not broken
    let dir = temp_dir();
    let path = dir.join("quoted.csv");
    let mut f = fs::File::create(&path).unwrap();
    writeln!(f, "id,description").unwrap();
    for i in 0..50 {
        // Every row has a quoted field with internal comma
        writeln!(f, "{},\"value with, embedded, commas and \"\"quotes\"\"\"", i).unwrap();
    }

    let out_dir = dir.join("out_quoted");
    fs::create_dir_all(&out_dir).unwrap();

    let cli = filesplit::cli::Cli {
        file: path,
        size: None,
        rows: Some(10),
        format: filesplit::cli::FileFormat::Csv,
        delimiter: ',',
        preserve_header: true,
        output: Some(out_dir.clone()),
        prefix: None,
        suffix: None,
        digits: 4,
        report: None,
        quiet: true,
        verbose: false,
    };

    let summary = filesplit::splitter::run(cli).unwrap();
    assert_eq!(summary.total_rows_written, 50);

    let outputs = gather_outputs(&out_dir, "quoted", ".csv");
    let total_data = count_csv_data_rows(&outputs);
    assert_eq!(total_data, 50);

    // Re-parse all outputs to verify CSV validity
    for output in &outputs {
        let mut rdr = csv::Reader::from_path(output).unwrap();
        for r in rdr.records() {
            let rec = r.expect("should parse cleanly");
            assert_eq!(rec.len(), 2, "each row must have exactly 2 fields");
        }
    }

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_size_parsing() {
    use filesplit::format::parse_size;
    assert_eq!(parse_size("1KB").unwrap(), 1024);
    assert_eq!(parse_size("1MB").unwrap(), 1024 * 1024);
    assert_eq!(parse_size("1GB").unwrap(), 1024 * 1024 * 1024);
    assert_eq!(parse_size("500").unwrap(), 500);
    assert!(parse_size("bad").is_err());
}

#[test]
fn test_json_report_written() {
    let dir = temp_dir();
    let input = write_csv(&dir, "report_test.csv", 100);
    let out_dir = dir.join("out_report");
    fs::create_dir_all(&out_dir).unwrap();
    let report_path = dir.join("report.json");

    let cli = filesplit::cli::Cli {
        file: input,
        size: None,
        rows: Some(30),
        format: filesplit::cli::FileFormat::Csv,
        delimiter: ',',
        preserve_header: true,
        output: Some(out_dir),
        prefix: None,
        suffix: None,
        digits: 4,
        report: Some(report_path.clone()),
        quiet: true,
        verbose: false,
    };

    filesplit::splitter::run(cli).unwrap();

    assert!(report_path.exists(), "report.json should exist");
    let content = fs::read_to_string(&report_path).unwrap();
    let v: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert!(v["total_chunks"].as_u64().unwrap() > 0);
    assert_eq!(v["total_rows_written"].as_u64().unwrap(), 100);

    fs::remove_dir_all(&dir).ok();
}
