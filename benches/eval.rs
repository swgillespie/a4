// Copyright 2021-2022 Sean Gillespie.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use a4::{eval, position::Position};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("complex-position-eval", |b| {
        let pos =
            Position::from_fen("rn1q1rk1/pbp1bppp/4pn2/1p6/P1pP4/5NP1/1PQ1PPBP/RNB2RK1 w - - 1 9")
                .unwrap();
        b.iter(|| {
            let pos = black_box(&pos);
            eval::evaluate(pos)
        });
    });

    c.bench_function("endgame-eval", |b| {
        let pos = Position::from_fen("5Q2/8/4k1p1/4p2p/3r3P/6KP/8/8 w - - 15 50").unwrap();
        b.iter(|| {
            let pos = black_box(&pos);
            eval::evaluate(pos)
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
