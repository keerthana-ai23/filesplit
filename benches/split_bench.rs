use criterion::{criterion_group, criterion_main, Criterion};
use std::fs;
use tempfile::TempDir;

fn generate_large_csv(rows: usize) -> (TempDir, String) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("bench.csv");
    let mut content = String::from("id,name,value,category\n");
    for i in 0..rows {
        content.push_str(&format!("{},\"User {}\",{:.4},category_{}\n", i, i, i as f64 * 1.1, i % 10));
    }
    fs::write(&path, &content).unwrap();
    (dir, path.to_string_lossy().to_string())
}

fn bench_split_by_rows(c: &mut Criterion) {
    let (_dir, input) = generate_large_csv(50_000);

    c.bench_function("split_50k_rows_by_1000", |b| {
        b.iter(|| {
            let out = TempDir::new().unwrap();
            use filesplit::splitter::{SplitMode, SplitterConfig, run_split};
            use filesplit::progress::SplitSummary;
            let mut summary = SplitSummary::new(false);
            let cfg = SplitterConfig {
                input_path: input.clone(),
                output_dir: out.path().to_string_lossy().to_string(),
                prefix: "bench".to_string(),
                mode: SplitMode::ByRows(1000),
                preserve_header: true,
                dry_run: false,
            };
            run_split(cfg, &mut summary).unwrap();
        })
    });
}

criterion_group!(benches, bench_split_by_rows);
criterion_main!(benches);
