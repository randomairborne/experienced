use std::collections::HashMap;

use criterion::{Criterion, criterion_group, criterion_main};
use simpleinterpolation::Interpolation;

pub fn criterion_benchmark(c: &mut Criterion) {
    let interp = Interpolation::new("{interp}").unwrap();
    let mut data = HashMap::new();
    data.insert(
        "interp".into(),
        include_str!("very_long_uninterpolated.txt").into(),
    );
    c.bench_function("all interpolation", |b| b.iter(|| interp.render(&data)));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
