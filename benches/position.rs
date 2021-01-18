// Copyright 2021 Sean Gillespie.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use gambit::core::{self, Move};
use gambit::Position;

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("quiet move clonemake 10x", |b| {
        let pos = Position::from_fen("8/8/4b3/8/2B5/8/8/8 w - - 0 1").unwrap();
        let mov = Move::quiet(core::C4, core::D5);
        b.iter(|| {
            let pos = black_box(&pos).clone();
            let mov = black_box(mov);
            for _ in 0..10 {
                let mut new = pos.clone();
                new.make_move(mov);
            }
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
