// Copyright 2021 Sean Gillespie.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

use crate::{
    core::*,
    eval::{evaluate, Value},
    movegen,
    position::Position,
    table::{self, NodeKind},
    threads,
};

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

/// Statistics about the search, reported to the caller upon termination of the search.
#[derive(Clone, Debug, Default)]
pub struct SearchStats {
    pub nodes_evaluated: u64,
    pub nodes_evaluated_per_depth: Vec<u64>,
}

#[derive(Clone, Debug)]
pub struct SearchResult {
    pub best_move: Move,
    pub best_score: Value,
    pub stats: SearchStats,
}

impl<'a: 'b, 'b> Searcher<'a, 'b> {
    fn new(options: &'a SearchOptions) -> Searcher<'a, 'b> {
        Searcher {
            nodes_evaluated: 0,
            search_start_time: Instant::now(),
            options,
        }
    }

    fn search(&mut self, pos: &Position, depth: u32) -> Option<(Move, Value)> {
        let alpha = Value::mated_in(0);
        let beta = Value::mate_in(0);
        let score = self.alpha_beta(pos, alpha, beta, depth);
        // If this search was cut short for any reason, we can't trust the alpha, beta, or score that we ended up with.
        if !self.can_continue_search() {
            return None;
        }

        let best_move = table::query(&pos)
            .expect("t-table miss after search?")
            .best_move()
            .expect("search thinks that root node is an all-node?");
        Some((best_move, score))
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
        // First, we order our moves so that we maximizes the chances of good moves being searched first.
        order_moves(pos, &mut moves);
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

    /// A quiesence search to terminate a search. The goal of the q-search is to only terminate the search at a
    /// position that is "quiet" and doesn't have any tactical possibilities. If we don't do so, the "horizon effect"
    /// can lead a4 into terminating a search at highly vulnerable situations.
    ///
    /// Consider a search that reaches its depth limit at a move where a queen takes a pawn that is defended by another
    /// pawn. We can't simply terminate the search there - we must continue evaluations until captures are complete,
    /// otherwise we will not see that our queen is lost.
    fn quiesce(&mut self, pos: &Position, mut alpha: Value, beta: Value) -> Value {
        tracing::debug!("initiating qsearch");
        self.nodes_evaluated += 1;
        // The "stand pat" score is a lower bound to how bad this position is. We're interested in finding refutations
        // to this position that drop this lower bound.
        let mut stand_pat = evaluate(pos);
        if stand_pat >= beta {
            // There exists a refutation in a sibling node - no point seaerching this.
            return beta;
        }
        if alpha < stand_pat {
            alpha = stand_pat;
        }

        let mut moves = Vec::new();
        movegen::generate_moves(pos.side_to_move(), pos, &mut moves);
        moves.retain(|&m| pos.is_legal_given_pseudolegal(m));
        moves.retain(|&m| m.is_capture());
        tracing::debug!("pos: {}", pos.as_fen());
        if tracing::enabled!(tracing::Level::DEBUG) {
            for mov in &moves {
                tracing::debug!("  capture: {}", mov.as_uci());
            }
        }

        if moves.len() == 0 {
            return if pos.side_to_move() == Color::Black {
                -stand_pat
            } else {
                stand_pat
            };
        }

        for capture in moves {
            if !self.can_continue_search() {
                return alpha;
            }

            let mut child = pos.clone();
            child.make_move(capture);
            stand_pat = -self.quiesce(&child, -beta, -alpha);
            if stand_pat >= beta {
                return beta;
            }
            if stand_pat >= alpha {
                alpha = stand_pat;
            }
        }

        alpha
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

/// Performs move ordering for a list of legal moves from a given position. Move ordering is crucial
/// for alpha-beta search. It is our best defense against combinatorial explosion of the state space
/// of chess.
///
/// This function heuristically orders all moves in order of how good they appear to be, without searching
/// the tree of moves directly.
///
/// Note that the hash move is not included here, since the searcher handles that already.
fn order_moves(pos: &Position, moves: &mut [Move]) {
    // For the purposes of move ordering, we derive a total order of moves by ranking them
    // by their static exchange scores. Static exchange generally refers to captures, but for move
    // ordering we'll also consider promotions to count for a score.
    //
    // We'll drive a move score for every move and use that as the sorting key.
    fn move_score(pos: &Position, mov: Move) -> i32 {
        match mov {
            // En-passant is an annoying edge case in everything, SEE is no exception. Put it before
            // the quiet moves but don't consider it particularly good.
            mov if mov.is_en_passant() => 1,
            // TODO(swgillespie) - This probably overestimates the value of promotion captures...
            mov if mov.is_capture() && mov.is_promotion() => {
                mov.promotion_piece().value() - 1
                    + static_exchange_evaluation(pos, mov.destination())
            }
            mov if mov.is_capture() => static_exchange_evaluation(pos, mov.destination()),
            mov if mov.is_promotion() => mov.promotion_piece().value() - 1,
            _ => 0,
        }
    }

    // TODO(swgillespie) look at checks first too?
    moves.sort_by_cached_key(|&mov| -move_score(pos, mov));
}

fn static_exchange_evaluation(pos: &Position, target: Square) -> i32 {
    let mut value = 0;
    if let Some(attacker) = smallest_attacker(pos, target) {
        let target_piece = pos.piece_at(target).unwrap();
        let mut child = pos.clone();
        let mov = Move::capture(attacker, target);
        child.make_move(mov);
        value = target_piece.kind.value() - static_exchange_evaluation(&child, target);
    }

    value
}

fn smallest_attacker(pos: &Position, target: Square) -> Option<Square> {
    let attackers = pos.squares_attacking(pos.side_to_move(), target);
    if attackers.is_empty() {
        return None;
    }

    let mut values: Vec<(Square, PieceKind)> = attackers
        .into_iter()
        .map(|sq| (sq, pos.piece_at(sq).unwrap().kind))
        .collect();

    values.sort_by_key(|(_, kind)| kind.value());
    return values.first().map(|(sq, _)| sq).cloned();
}

pub fn search(pos: &Position, options: &SearchOptions) -> SearchResult {
    tracing::info!("initiating search ({:?})", options);
    let mut stats = SearchStats::default();
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

        let search_start = Instant::now();
        if let Some((best_move, best_score)) = searcher.search(pos, depth) {
            let search_time = Instant::now().duration_since(search_start);
            node_count += searcher.nodes_evaluated;
            stats.nodes_evaluated += searcher.nodes_evaluated;
            stats
                .nodes_evaluated_per_depth
                .push(searcher.nodes_evaluated);
            current_best_move = best_move;
            current_best_score = best_score;
            let nps = searcher.nodes_evaluated as f64 / search_time.as_secs_f64();
            let pv = table::get_pv(pos, depth);
            tracing::debug!("pv: {:?}", pv);
            if threads::get_worker_id() == Some(0) {
                // TODO(swgillespie) - seldepth, how far did the qsearch go
                let pv_str = pv
                    .into_iter()
                    .map(|mov| mov.as_uci())
                    .collect::<Vec<_>>()
                    .join(" ");
                println!(
                    "info depth {} nodes {} nps {} pv {} score {}",
                    depth,
                    searcher.nodes_evaluated,
                    nps.floor() as i64,
                    pv_str,
                    current_best_score.as_uci(),
                );
            }
        }
    }

    if threads::get_worker_id() == Some(0) {
        println!("bestmove {}", current_best_move.as_uci());
    }

    SearchResult {
        best_move: current_best_move,
        best_score: current_best_score,
        stats,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{movegen::generate_moves, position::Position};

    #[test]
    fn see_pawn_exchange_bad_for_player() {
        let pos = Position::from_fen("8/6p1/1R3b2/8/8/2B5/8/5r2 w - - 0 1").unwrap();
        // White to move, white threatens f6 and initiates an exchange.
        let predicted_yield = static_exchange_evaluation(&pos, F6);

        // White trades a bishop and a rook (8) for a pawn and a bishop (4), a loss of 4.
        assert_eq!(predicted_yield, -4);
    }

    #[test]
    fn see_exchange_good_for_player() {
        let pos = Position::from_fen("8/r2q4/8/8/6B1/8/3Q4/8 w - - 0 1").unwrap();
        // White to move, white threatens Bxd7 and initiates an exchange.
        let predicted_yield = static_exchange_evaluation(&pos, D7);

        // White trades a bishop (3) for a queen and a rook (14), for a win of 11.
        assert_eq!(predicted_yield, 11);
    }

    #[test]
    fn move_ordering_good_captures_first() {
        let pos = Position::from_fen("5b2/8/3r2r1/2P5/5B2/8/3Q4/8 w - - 0 1").unwrap();
        let mut moves = Vec::new();
        generate_moves(pos.side_to_move(), &pos, &mut moves);
        moves.retain(|&m| pos.is_legal_given_pseudolegal(m));

        order_moves(&pos, &mut moves);
        assert_eq!(moves.first().cloned().unwrap(), Move::capture(C5, D6));
    }
}
