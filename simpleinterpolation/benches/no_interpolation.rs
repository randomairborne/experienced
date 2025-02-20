use std::collections::HashMap;

use criterion::{Criterion, criterion_group, criterion_main};
use simpleinterpolation::Interpolation;

pub fn criterion_benchmark(c: &mut Criterion) {
    let interp = Interpolation::new(include_str!("very_long_uninterpolated.txt")).unwrap();
    let empty_hashmap = HashMap::new();
    c.bench_function("no interpolation", |b| {
        b.iter(|| interp.render(&empty_hashmap))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
