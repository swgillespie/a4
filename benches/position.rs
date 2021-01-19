// Copyright 2021 Sean Gillespie.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use gambit::core::{self, Color, Move};
use gambit::movegen;
use gambit::Position;

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("quiet-move-clonemake", |b| {
        let pos = Position::from_fen("8/8/4b3/8/2B5/8/8/8 w - - 0 1").unwrap();
        let mov = Move::quiet(core::C4, core::D5);
        b.iter(|| {
            let mut pos = black_box(&pos).clone();
            let mov = black_box(mov);
            pos.make_move(mov);
        });
    });

    c.bench_function("pawn-movegen", |b| {
        let pos = Position::from_fen(
            "r3k2r/p1ppqpb1/bn2pnp1/3PN3/Pp2P3/2N2Q1p/1PPBBPPP/R3K2R b KQkq a3 0 1",
        )
        .unwrap();
        b.iter(|| {
            let mut moves = Vec::new();
            movegen::generate_pawn_moves(black_box(Color::Black), black_box(&pos), &mut moves);
        });
    });

    c.bench_function("kiwipete-movegen-all", |b| {
        let pos = Position::from_fen(
            "r3k2r/p1ppqpb1/bn2pnp1/3PN3/Pp2P3/2N2Q1p/1PPBBPPP/R3K2R b KQkq a3 0 1",
        )
        .unwrap();
        b.iter(|| {
            let mut moves = Vec::new();
            movegen::generate_moves(black_box(Color::Black), black_box(&pos), &mut moves);
        });
    });

    c.bench_function("kiwipete-movegen-quiet", |b| {
        let pos = Position::from_fen(
            "r3k2r/p1ppqpb1/bn2pnp1/3PN3/Pp2P3/2N2Q1p/1PPBBPPP/R3K2R b KQkq a3 0 1",
        )
        .unwrap();
        b.iter(|| {
            let mut moves = Vec::new();
            movegen::generate_moves(black_box(Color::Black), black_box(&pos), &mut moves);
            moves.retain(|m| m.is_quiet());
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
