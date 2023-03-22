use criterion::{black_box, criterion_group, criterion_main, Criterion};

use ck_dodo::curve::field::Fp;

fn fp_benchmark(c: &mut Criterion) {
    let a = Fp::constant();
    c.bench_function("Fp *=", |b| b.iter(|| *(&mut black_box(a)) *= black_box(a)));
}

criterion_group!(benches, fp_benchmark);

criterion_main!(benches);
