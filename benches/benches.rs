use criterion::{black_box, criterion_group, criterion_main, Criterion};

use ck_dodo::curve::field::Fp;

fn fp_benchmark(c: &mut Criterion) {
    let a = Fp::constant();
    c.bench_function("Fp add", |b| b.iter(|| black_box(a).add(black_box(a))));
}

criterion_group!(benches, fp_benchmark);

criterion_main!(benches);
