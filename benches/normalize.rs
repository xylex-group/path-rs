use criterion::{Criterion, black_box, criterion_group, criterion_main};
use path_rs::{ExpandOptions, expand_input, normalize};

fn bench_normalize(c: &mut Criterion) {
    c.bench_function("normalize_short", |b| {
        b.iter(|| normalize(black_box("foo/./bar/../baz")).unwrap())
    });

    let long = "a/".repeat(64) + "file.txt";
    c.bench_function("normalize_long", |b| {
        b.iter(|| normalize(black_box(long.as_str())).unwrap())
    });

    let opts = ExpandOptions {
        reject_undefined_variables: false,
        ..ExpandOptions::default()
    };
    c.bench_function("expand_tilde", |b| {
        b.iter(|| expand_input(black_box("~/projects/demo"), &opts).unwrap())
    });
}

criterion_group!(benches, bench_normalize);
criterion_main!(benches);
