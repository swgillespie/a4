// Copyright 2021 Sean Gillespie.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
use crate::core::*;
use crate::eval::{evaluate, Value};
use crate::movegen;
use crate::Position;

struct Searcher {
    nodes_evaluated: u32,
}

pub struct SearchResult {
    pub best_move: Move,
    pub best_score: Value,
    pub nodes_evaluated: u32,
}

impl Searcher {
    fn new() -> Searcher {
        Searcher { nodes_evaluated: 0 }
    }

    fn search(&mut self, pos: &Position, depth: u32) -> SearchResult {
        let mut best_move = Move::null();
        let mut seen_a_legal_move = false;
        let mut best_score = Value::mated_in(1);
        let mut alpha = best_score;
        let beta = -best_score;
        let mut moves = Vec::new();
        movegen::generate_moves(pos.side_to_move(), pos, &mut moves);
        for mov in moves {
            if !pos.is_legal_given_pseudolegal(mov) {
                continue;
            }

            let mut child_pos = pos.clone();
            child_pos.make_move(mov);
            let score = -self.alpha_beta(pos, -beta, -alpha, depth - 1);
            if score > alpha {
                alpha = score;
            }
            if score > best_score || !seen_a_legal_move {
                seen_a_legal_move = true;
                best_score = score;
                best_move = mov;
            }
        }

        SearchResult {
            best_move,
            best_score,
            nodes_evaluated: self.nodes_evaluated,
        }
    }

    fn alpha_beta(&mut self, pos: &Position, mut alpha: Value, beta: Value, depth: u32) -> Value {
        if depth == 0 {
            return self.quiesce(pos, alpha, beta);
        }

        let mut moves = Vec::new();
        movegen::generate_moves(pos.side_to_move(), pos, &mut moves);
        for mov in moves {
            if !pos.is_legal_given_pseudolegal(mov) {
                continue;
            }

            let mut child_pos = pos.clone();
            child_pos.make_move(mov);
            let score = -self.alpha_beta(pos, -beta, -alpha, depth - 1);
            if score >= beta {
                return beta;
            }

            if score > alpha {
                alpha = score;
            }
        }

        alpha
    }

    fn quiesce(&mut self, pos: &Position, alpha: Value, beta: Value) -> Value {
        self.nodes_evaluated += 1;
        let value = evaluate(pos);
        if pos.side_to_move() == Color::Black {
            -value
        } else {
            value
        }
    }
}

pub fn search(pos: &Position, depth: u32) -> SearchResult {
    Searcher::new().search(pos, depth)
}
