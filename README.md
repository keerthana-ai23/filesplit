# filesplit

**Safe, streaming CSV and text file splitter for data engineering pipelines.**

Splits large files by size or row count while **preserving row boundaries** and **correctly handling quoted CSV values** — no corrupted chunks, no mid-row cuts.

---

## Features

| Feature | Details |
|---|---|
| ✅ Row-safe splitting | Never cuts a row in the middle |
| ✅ Quoted field handling | Correctly parses `"field with, comma"` via the `csv` crate |
| ✅ Size-based splitting | `--size 1GB`, `500MB`, `100KB`, etc. |
| ✅ Row-count splitting | `--rows 500000` |
| ✅ Header preservation | Copies the header into every split file |
| ✅ Text file support | Works on `.log`, `.ndjson`, `.txt` etc. |
| ✅ TSV support | `--format tsv` or auto-detected from `.tsv` extension |
| ✅ Streaming | Constant memory usage — handles files larger than RAM |
| ✅ Progress bar | Real-time byte-level progress |
| ✅ JSON report | `--report summary.json` for pipeline metadata |
| ✅ Custom naming | `--prefix`, `--suffix`, `--digits`, `--output` |

---

## Install

### From source (requires Rust 1.75+)

```bash
git clone https://github.com/you/filesplit
cd filesplit
cargo build --release
# Binary at: ./target/release/filesplit
sudo cp target/release/filesplit /usr/local/bin/
```

### Quick dev run

```bash
cargo run -- -f large.csv --rows 500000
```

---

## Usage

```
filesplit [OPTIONS] -f <FILE> (--size <SIZE> | --rows <N>)
```

### Options

```
  -f, --file <FILE>          Input file to split
      --size <SIZE>          Max size per chunk (e.g. 100MB, 1GB, 500KB)
      --rows <N>             Max rows per chunk (header not counted)
      --format <FORMAT>      auto | csv | tsv | text  [default: auto]
  -d, --delimiter <CHAR>     Field delimiter  [default: ,]
      --preserve-header      Copy header into every split file  [default: true]
  -o, --output <DIR>         Output directory  [default: same as input]
      --prefix <PREFIX>      Output filename prefix  [default: input stem]
      --suffix <EXT>         Output file extension  [default: input extension]
      --digits <N>           Zero-padding width for part numbers  [default: 4]
      --report <FILE>        Write JSON summary report to this path
  -q, --quiet                Suppress progress bar
  -v, --verbose              Print per-chunk details
  -h, --help                 Print help
  -V, --version              Print version
```

---

## Examples

### Split a CSV by row count, preserving the header

```bash
filesplit -f large.csv --rows 500000 --preserve-header
# → large_part_0001.csv  (500,000 rows + header)
# → large_part_0002.csv  (500,000 rows + header)
# → large_part_0003.csv  (remaining rows + header)
```

### Split by file size

```bash
filesplit -f huge.csv --size 1GB
```

### Custom output location and prefix

```bash
filesplit -f /data/sales.csv \
  --rows 1000000 \
  --output /data/chunks/ \
  --prefix sales_chunk \
  --digits 5
# → /data/chunks/sales_chunk_00001.csv
# → /data/chunks/sales_chunk_00002.csv
```

### Split a log file by line count

```bash
filesplit -f server.log --rows 100000 --format text --suffix .log
# → server_part_0001.log
# → server_part_0002.log
```

### TSV file

```bash
filesplit -f export.tsv --size 250MB
```

### Write a JSON report for pipeline metadata

```bash
filesplit -f events.csv --rows 500000 --report split_report.json
```

`split_report.json` example:

```json
{
  "input_file": "events.csv",
  "input_size_bytes": 2147483648,
  "input_size_human": "2 GiB",
  "format": "CSV",
  "split_mode": "500000 rows/chunk",
  "output_dir": ".",
  "total_chunks": 5,
  "total_rows_written": 2500000,
  "elapsed_seconds": 12.43,
  "chunks": [
    { "part": 1, "path": "events_part_0001.csv", "rows": 500000, "bytes": 429496729, "bytes_human": "409.6 MiB" },
    ...
  ]
}
```

---

## Why not just `split(1)` or `awk`?

| Tool | Row-safe? | Quoted CSV? | Header preservation? | Progress? |
|---|---|---|---|---|
| `split -l` | ✅ | ❌ breaks on quoted newlines | ❌ | ❌ |
| `awk` | ✅ | ❌ fragile | manual | ❌ |
| `head`/`tail` | ❌ | ❌ | ❌ | ❌ |
| **filesplit** | ✅ | ✅ | ✅ | ✅ |

---

## Architecture

```
src/
├── main.rs        — Entry point, banner, error handling
├── cli.rs         — Clap derive CLI definitions
├── splitter.rs    — Core streaming split logic (CSV + text)
├── format.rs      — Size string parsing & formatting
├── report.rs      — Summary data structures + terminal/JSON output
└── error.rs       — Typed error enum (thiserror)

tests/
└── integration_tests.rs — End-to-end tests with real files
```

### Streaming design

- Uses `BufReader` with an 8 MB read buffer — memory usage stays constant regardless of file size
- CSV splitting: uses the `csv` crate reader which correctly handles RFC 4180 quoted fields, embedded newlines, and escaped quotes
- Rolling: when a chunk hits its limit, the current `BufWriter` is flushed and a new file is opened — no intermediate buffering of entire chunks

---

## Running Tests

```bash
cargo test
```

Tests cover:
- CSV row-count splitting (data integrity)
- CSV size-based splitting (valid CSV output)
- Text line splitting
- Quoted field correctness (embedded commas, escaped quotes)
- Size string parsing
- JSON report generation

---

## License

MIT
