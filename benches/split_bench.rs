use criterion::{criterion_group, criterion_main, Criterion};
use std::fs;
use tempfile::TempDir;

fn generate_csv(rows: usize) -> (TempDir, String) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("bench.csv");
    let mut content = String::from("id,name,value\n");
    for i in 0..rows {
        content.push_str(&format!("{},\"User {}\",{}\n", i, i, i));
    }
    fs::write(&path, &content).unwrap();
    (dir, path.to_string_lossy().to_string())
}

fn bench_rows(c: &mut Criterion) {
    let (_dir, input) = generate_csv(10_000);
    c.bench_function("split_10k_by_1000_rows", |b| {
        b.iter(|| {
            let out = TempDir::new().unwrap();
            std::process::Command::new("cargo")
                .args(["run","--release","--","-f",&input,"--rows","1000","-o",out.path().to_str().unwrap()])
                .output().ok();
        })
    });
}

criterion_group!(benches, bench_rows);
criterion_main!(benches);
