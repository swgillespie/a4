//! Move ordering for search routines.
//!
//! Alpha-beta searches perform best when high-quality moves are searched first. Since our move generator generates
//! moves in no particular order, it is up to the routines in this module to order the moves in such a way that moves
//! that are most likely to be good are searched first, so that the alpha-beta search can cutoff the remaining nodes
//! as quickly as possible.

use crate::{
    core::{Move, PieceKind, Square},
    position::Position,
};

/// Performs move ordering for a list of legal moves from a given position. Move ordering is crucial
/// for alpha-beta search. It is our best defense against combinatorial explosion of the state space
/// of chess.
///
/// This function heuristically orders all moves in order of how good they appear to be, without searching
/// the tree of moves directly.
///
/// Note that the hash move is not included here, since the searcher handles that already.
pub fn order_moves(pos: &Position, moves: &mut [Move]) {
    fn see_weight(pos: &Position, mov: Move) -> i32 {
        if mov.is_capture() {
            let child_pos = pos.clone_and_make_move(mov);
            let captured_piece_value = pos
                .piece_at(mov.destination())
                .expect("illegal move given to order moves")
                .kind
                .value();

            // For promo captures, we "gain" material points from turning the pawn into another piece.
            let promotion_value = if mov.is_promotion() {
                mov.promotion_piece().value() - 1
            } else {
                0
            };
            return captured_piece_value
                + promotion_value
                + static_exchange_evaluation(&child_pos, mov.destination());
        }

        // Things that aren't captures have a weight of zero.
        return 0;
    }

    // We are particularly interested in investigating captures first.
    let (captures, quiet) = partition_by(moves, |mov| mov.is_capture());

    // Captures resulting in check are particularly interesting.
    let (_, _) = partition_by(captures, |mov| {
        let mut child_pos = pos.clone();
        child_pos.make_move(mov);
        child_pos.is_check(pos.side_to_move())
    });

    // Quiet moves resulting in checks are also interesting.
    let (_, _) = partition_by(quiet, |mov| {
        let mut child_pos = pos.clone();
        child_pos.make_move(mov);
        child_pos.is_check(pos.side_to_move())
    });

    captures.sort_by_cached_key(|&mov| see_weight(pos, mov));
}

/// Partitions the move array such that all moves that satisfy the given predicate are placed at the start of the array
/// and all moves that don't are placed at the end.
///
/// The standard library function `partition_point` can be used to efficiently query the index that the predicate
/// becomes false.
fn partition_by<F: FnMut(Move) -> bool>(
    moves: &mut [Move],
    mut func: F,
) -> (&mut [Move], &mut [Move]) {
    let mut i = 0;
    let mut j = moves.len() - 1;
    loop {
        while func(moves[i]) && i < moves.len() {
            i += 1;
        }

        while !func(moves[j]) && j > 0 {
            j -= 1;
        }

        if i >= j {
            break;
        }

        // SAFETY: i always is bounded above by moves.len() - 1, and j is always bounded below by 0.
        unsafe {
            moves.swap_unchecked(i, j);
        }
    }

    moves.split_at_mut(i)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{core::*, movegen::generate_moves, position::Position};

    #[test]
    fn partition_by_captures() {
        let pos = Position::from_fen("4k3/8/3p4/8/8/2P5/3R2r1/1K6 w - - 0 1").unwrap();
        let mut moves = Vec::new();
        generate_moves(pos.side_to_move(), &pos, &mut moves);

        partition_by(&mut moves, |mov| mov.is_capture());
        let idx = moves.partition_point(|mov| mov.is_capture());
        let (left, right) = moves.split_at(idx);
        assert!(left.iter().all(|mov| mov.is_capture()));
        assert!(right.iter().all(|mov| !mov.is_capture()));
    }

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
    fn see_exchange_queen() {
        let pos = Position::from_fen("5b2/8/3r2r1/2P5/5B2/8/3Q4/8 w - - 0 1").unwrap();
        let predicted_yield = static_exchange_evaluation(&pos, D6);

        // Rook (5) - Pawn (1) + Rook (5) - Bishop (3) + Bishop(3) = 9
        assert_eq!(predicted_yield, 9);
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

    #[test]
    #[ignore]
    fn move_ordering_real_world() {
        let pos =
            Position::from_fen("r1bqkb1r/ppp3pp/2n2p2/3np3/2BP4/5N2/PPP2PPP/RNBQ1RK1 w kq - 0 7")
                .unwrap();
        let mut moves = Vec::new();
        generate_moves(pos.side_to_move(), &pos, &mut moves);
        moves.retain(|&m| pos.is_legal_given_pseudolegal(m));

        order_moves(&pos, &mut moves);
        assert_eq!(moves.first().cloned().unwrap(), Move::capture(D4, E5));
    }
}