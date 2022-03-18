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
use crate::position::Position;
use crate::table::{self, NodeKind};
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

/// Options for a search.
#[derive(Default, Debug)]
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

    fn search(&mut self, pos: &Position, depth: u32) -> SearchResult {
        let alpha = Value::mated_in(0);
        let beta = Value::mate_in(0);
        let score = self.alpha_beta(pos, alpha, beta, depth);
        let best_move = table::query(&pos)
            .expect("t-table miss after search?")
            .best_move()
            .expect("search thinks that root node is an all-node?");
        SearchResult {
            best_move,
            best_score: score,
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

        // Consult the transposition table. Have we seen this position before and, if so, does it produce a cutoff?
        // If so, there's no need to continue processing this position.
        let (mut hash_move, cutoff_value) =
            self.consider_transposition(pos, &mut alpha, beta, depth);
        if let Some(cutoff) = cutoff_value {
            return cutoff;
        }

        //
        // Step 1 - Consider and evaluate the hash move.
        //

        // Apply a legality test. In the event of t-table collisions, the hash move might not be a legal move.
        hash_move = hash_move.and_then(|mov| if pos.is_legal(mov) { Some(mov) } else { None });

        // Keep track if any move improved alpha. If so, this is a PV node.
        let mut improved_alpha = false;
        if let Some(hash_move) = hash_move {
            let mut hash_pos = pos.clone();
            hash_pos.make_move(hash_move);
            let value = -self.alpha_beta(&hash_pos, -beta, -alpha, depth - 1);
            if value >= beta {
                table::record_cut(pos, hash_move, depth, value);
                return beta.step();
            }

            if value > alpha {
                improved_alpha = true;
                table::record_pv(pos, hash_move, depth, value);
                alpha = value;
            }
        }

        //
        // Step 2 - Generate moves and scan the position.
        //

        let mut moves = Vec::new();
        movegen::generate_moves(pos.side_to_move(), pos, &mut moves);
        moves.retain(|&m| pos.is_legal_given_pseudolegal(m));
        if moves.len() == 0 {
            // No legal moves available. Are we in check?
            let value = if pos.is_check(pos.side_to_move()) {
                // We lost.
                Value::mated_in(0)
            } else {
                // We've drawn.
                Value::new(0)
            };

            table::record_pv(pos, Move::null(), depth, value);
            return value.step();
        }

        // We have at least one legal move available to us, so let's play.
        for mov in moves {
            let mut child = pos.clone();
            child.make_move(mov);
            let value = -self.alpha_beta(&child, -beta, -alpha, depth - 1);
            if value >= beta {
                table::record_cut(pos, mov, depth, value);
                return beta.step();
            }

            if value > alpha {
                improved_alpha = true;
                table::record_pv(pos, mov, depth, value);
                alpha = value;
            }
        }

        if !improved_alpha {
            table::record_all(pos, depth, alpha);
        }

        alpha.step()
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

    fn consider_transposition(
        &self,
        pos: &Position,
        alpha: &mut Value,
        beta: Value,
        depth: u32,
    ) -> (Option<Move>, Option<Value>) {
        // The alpha-beta function in this searcher is designed to exploit the transposition table to take the best
        // known path through the game tree. The transposition table serves two purposes:
        //   1. If the t-table records that we've already done a really deep search for a particular position, we can
        //      use the t-table's exact results as the results of this search and avoid having to do a search entirely.
        //   2. If the t-table records that we've done a search for this position, but it's not deep enough to serve
        //      this search, we can use its best move (or "hash move") to guide our search. We'll search that move
        //      before even generating moves for the current position, in the hopes that the hash move either fails high
        //      or produces a really high alpha.
        let hash_move = if let Some(entry) = table::query(pos) {
            // Transposition table hit. We might not be able to use this hit, though:
            //    1. If the entry's depth is less than the depth we are currently searching at, we shouldn't
            //       use this entry since the search we are about to do is going to be higher fidelity.
            //    2. If the entry's best move isn't a legal move, then we probably had a collision in the t-table
            //       and shouldn't use it.
            //    3. If the entry is an all node, it doesn't even have a hash move. We can still try to fail low, but
            //       we won't get a hash move out of it.
            let hash_move = entry.best_move();
            if entry.depth() >= depth {
                // We can actually use this node! To guard against hash collisions, we do need to apply a legality test
                // on the hash move.
                if hash_move.is_none() || pos.is_legal(hash_move.unwrap()) {
                    // Either we don't have a hash move (all-node) or we do and it cut off. Either way, we get to avoid
                    // doing some work.
                    match entry.kind() {
                        NodeKind::PV(value) => {
                            // The last time we searched at this depth or greater, this move was a PV-node. This is the
                            // best case scenario; we know exactly what the score is. We don't have to search this subtree
                            // at all.
                            return (hash_move, Some(value.step()));
                        }
                        NodeKind::Cut(value) => {
                            // The last time we searched at this depth or greater, this move caused a beta cutoff. The score
                            // here is a lower-bound on the exact score of the node.
                            //
                            // If the lower bound is greater than beta, we don't need to search this node and can instead
                            // return beta.
                            if value >= beta {
                                return (hash_move, Some(value.step()));
                            }

                            // If the lower bound is greater than alpha, bump up alpha to match.
                            if value >= *alpha {
                                *alpha = value;
                            }

                            // Otherwise, we should search the hash move first - it'll probably cause a beta cutoff.
                        }
                        NodeKind::All(value) => {
                            // The last time we searched at this depth or greater, we searched all children of this node and
                            // none of them improved alpha. The score here is an upper-bound on the exact score of the node.
                            //
                            // If the upper bound is worse than alpha, we're not going to find anything better if we search
                            // here.
                            if value <= *alpha {
                                return (hash_move, Some(alpha.step()));
                            }

                            // Otherwise, we'll need to search everything, starting at the hash move.
                        }
                    }
                }
            }

            hash_move
        } else {
            None
        };

        (hash_move, None)
    }
}

pub fn search(pos: &Position, options: &SearchOptions) -> SearchResult {
    tracing::info!("initiating search ({:?})", options);
    let mut current_best_move = Move::null();
    let mut current_best_score = Value::mated_in(0);
    let start_time = Instant::now();
    let mut node_count = 0;
    for depth in 1..=options.depth {
        tracing::info!("beginning iterative search of depth {}", depth);
        let time_since_start = Instant::now().duration_since(start_time);
        if let Some(limit) = options.time_limit {
            if limit < time_since_start {
                break;
            }
        }
        let subsearch_opts = SearchOptions {
            time_limit: options
                .time_limit
                .map(|limit| limit.saturating_sub(time_since_start)),
            depth,
            hard_stop: options.hard_stop,
            node_limit: options
                .node_limit
                .map(|limit| limit.saturating_sub(node_count)),
        };

        let mut searcher = Searcher::new(&subsearch_opts);
        if !searcher.can_continue_search() {
            break;
        }

        let result = searcher.search(pos, depth);
        node_count += result.nodes_evaluated;
        current_best_move = result.best_move;
        current_best_score = result.best_score;
        let pv = table::get_pv(pos, depth);
        tracing::info!("pv: {:?}", pv);
    }

    SearchResult {
        best_move: current_best_move,
        best_score: current_best_score,
        nodes_evaluated: node_count,
    }
}
