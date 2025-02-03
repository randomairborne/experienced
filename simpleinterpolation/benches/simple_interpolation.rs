use std::collections::HashMap;

use criterion::{criterion_group, criterion_main, Criterion};
use simpleinterpolation::Interpolation;

pub fn criterion_benchmark(c: &mut Criterion) {
    let interp = Interpolation::new("simple {fill}").unwrap();
    let mut keys = HashMap::new();
    keys.insert("fill".into(), "interpolation".into());
    c.bench_function("simple interpolation", |b| b.iter(|| interp.render(&keys)));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
