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
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

/// Options for a search.
#[derive(Default)]
pub struct SearchOptions<'a> {
    /// Maximum amount of time to dedicate to this search.
    pub time_limit: Option<Duration>,

    /// Maximum amount of nodes to evaluate.
    pub node_limit: Option<u64>,

    /// Reference to a hard stop flag, which (if set) should immediately terminate the search.
    pub hard_stop: Option<&'a AtomicBool>,

    /// Maximum depth to search.
    pub depth: u32,
}

struct Searcher<'a, 'b> {
    search_start_time: Instant,
    nodes_evaluated: u64,
    options: &'a SearchOptions<'b>,
}

#[derive(Copy, Clone, Debug)]
pub struct SearchResult {
    pub best_move: Move,
    pub best_score: Value,
    pub nodes_evaluated: u64,
}

impl<'a: 'b, 'b> Searcher<'a, 'b> {
    fn new(options: &'a SearchOptions) -> Searcher<'a, 'b> {
        Searcher {
            nodes_evaluated: 0,
            search_start_time: Instant::now(),
            options,
        }
    }

    fn search(&mut self, pos: &Position, options: &SearchOptions) -> SearchResult {
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

            if !self.can_continue_search() {
                // TODO
            }

            let mut child_pos = pos.clone();
            child_pos.make_move(mov);
            let score = self.alpha_beta(pos, -beta, -alpha, options.depth - 1);
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
        // Two places that we check for search termination, inserted in the same place that a compiler would insert safepoints for preemption:
        //   1. Function entry blocks, so we can cut off trees that we are about to search if we are out of time
        //   2. Loop back edges, so we can cut off trees that we are partially in the process of searching
        if !self.can_continue_search() {
            return alpha;
        }

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

            if !self.can_continue_search() {
                return alpha;
            }
        }

        alpha
    }

    fn quiesce(&mut self, pos: &Position, _alpha: Value, _beta: Value) -> Value {
        self.nodes_evaluated += 1;
        let value = evaluate(pos);
        if pos.side_to_move() == Color::Black {
            -value
        } else {
            value
        }
    }

    fn can_continue_search(&self) -> bool {
        if let Some(limit) = self.options.time_limit {
            if Instant::now().saturating_duration_since(self.search_start_time) > limit {
                return false;
            }
        }

        if let Some(limit) = self.options.node_limit {
            if self.nodes_evaluated > limit {
                return false;
            }
        }

        if let Some(ptr) = self.options.hard_stop {
            if ptr.load(Ordering::Acquire) {
                return false;
            }
        }

        true
    }
}

pub fn search(pos: &Position, options: &SearchOptions) -> SearchResult {
    Searcher::new(options).search(pos, options)
}
