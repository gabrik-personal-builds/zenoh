#[macro_use]
extern crate criterion;
extern crate rand;

use criterion::{Criterion, black_box};

use zenoh_protocol::io::WBuf;

fn bench_foo((v, buf): (u64, &mut WBuf)) {  
  buf.write_zint(v);
}

fn criterion_benchmark(c: &mut Criterion) {
    let mut buf = WBuf::new(64);     
    let rs3: [u64;3] = [u64::from(rand::random::<u8>()), u64::from(rand::random::<u8>()), u64::from(rand::random::<u8>())];     
    let _rs2: [u64;2] = [u64::from(rand::random::<u8>()), u64::from(rand::random::<u8>())];
    let _ns: [u64;4] = [0; 4];
    let _len = String::from("u8");

    c.bench_function("bench_foo u8", |b| b.iter(|| {
      let _ = bench_foo(black_box((rs3[0], &mut buf)));
      buf.clear();
      }));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
