use criterion::{Criterion, black_box, criterion_group, criterion_main};
use path_rs::{ListOptions, list};
use std::fs;
use tempfile::tempdir;

fn populate(dir: &std::path::Path, count: usize) {
    for i in 0..count {
        fs::write(dir.join(format!("f{i}.txt")), b"x").unwrap();
    }
}

fn bench_listing(c: &mut Criterion) {
    let small = tempdir().unwrap();
    populate(small.path(), 100);
    let large = tempdir().unwrap();
    populate(large.path(), 10_000);

    let opts = ListOptions::new().recursive(false);

    c.bench_function("list_100", |b| {
        b.iter(|| list(black_box(small.path()), black_box(&opts)).unwrap())
    });

    c.bench_function("list_10000", |b| {
        b.iter(|| list(black_box(large.path()), black_box(&opts)).unwrap())
    });

    // Keep dirs alive for the duration of the bench group.
    let _keep = (small.keep(), large.keep());
}

criterion_group!(benches, bench_listing);
criterion_main!(benches);
