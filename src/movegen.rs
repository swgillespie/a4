// Copyright 2021 Sean Gillespie.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
use crate::core::*;
use crate::Position;

fn generate_pawn_moves(us: Color, pos: &Position, moves: &mut Vec<Move>) {
    let them = us.toggle();
    let their_pieces = pos.pieces(them);
    let our_pieces = pos.pieces(us);
    let all_pieces = their_pieces.or(our_pieces);
    let empty_squares = all_pieces.not();
    let (up, down, up_left, up_right, promo_rank, start_rank) = if us == Color::White {
        (
            Direction::North,
            Direction::South,
            Direction::NorthWest,
            Direction::NorthEast,
            SquareSet::all().rank(RANK_8),
            SquareSet::all().rank(RANK_2),
        )
    } else {
        (
            Direction::South,
            Direction::North,
            Direction::SouthWest,
            Direction::SouthEast,
            SquareSet::all().rank(RANK_1),
            SquareSet::all().rank(RANK_7),
        )
    };
    let rank_below_promo = promo_rank.shift(down);
    let our_pawns = pos.pawns(us);

    // Single and double pawn pushes, not counting promotions.
    {
        let single_pushes = our_pawns
            .and(rank_below_promo.not())
            .shift(up)
            .and(empty_squares);
        let double_pushes = single_pushes
            .and(start_rank.shift(up))
            .shift(up)
            .and(empty_squares);
        for target in single_pushes {
            moves.push(Move::quiet(target.towards(down), target));
        }
        for target in double_pushes {
            moves.push(Move::double_pawn_push(
                target.towards(down).towards(down),
                target,
            ));
        }
    }

    // Promotions, both captures and not.
    let pawns_near_promo = our_pawns.and(rank_below_promo);
    if !pawns_near_promo.is_empty() {
        let up_left_promo = pawns_near_promo.shift(up_left).and(their_pieces);
        let up_right_promo = pawns_near_promo.shift(up_right).and(their_pieces);
        let up_promo = pawns_near_promo.shift(up).and(empty_squares);
        for target in up_left_promo {
            moves.push(Move::promotion_capture(
                target.towards(up_left.reverse()),
                target,
                PieceKind::Bishop,
            ));
            moves.push(Move::promotion_capture(
                target.towards(up_left.reverse()),
                target,
                PieceKind::Knight,
            ));
            moves.push(Move::promotion_capture(
                target.towards(up_left.reverse()),
                target,
                PieceKind::Rook,
            ));
            moves.push(Move::promotion_capture(
                target.towards(up_left.reverse()),
                target,
                PieceKind::Queen,
            ));
        }

        for target in up_right_promo {
            moves.push(Move::promotion_capture(
                target.towards(up_right.reverse()),
                target,
                PieceKind::Bishop,
            ));
            moves.push(Move::promotion_capture(
                target.towards(up_right.reverse()),
                target,
                PieceKind::Knight,
            ));
            moves.push(Move::promotion_capture(
                target.towards(up_right.reverse()),
                target,
                PieceKind::Rook,
            ));
            moves.push(Move::promotion_capture(
                target.towards(up_right.reverse()),
                target,
                PieceKind::Queen,
            ));
        }

        for target in up_promo {
            moves.push(Move::promotion_capture(
                target.towards(up.reverse()),
                target,
                PieceKind::Bishop,
            ));
            moves.push(Move::promotion_capture(
                target.towards(up.reverse()),
                target,
                PieceKind::Knight,
            ));
            moves.push(Move::promotion_capture(
                target.towards(up.reverse()),
                target,
                PieceKind::Rook,
            ));
            moves.push(Move::promotion_capture(
                target.towards(up.reverse()),
                target,
                PieceKind::Queen,
            ));
        }
    }
}

pub fn generate_moves(us: Color, pos: &Position, moves: &mut Vec<Move>) {
    generate_pawn_moves(us, pos, moves);
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::generate_moves;
    use crate::core::*;
    use crate::Position;

    fn assert_moves_generated(fen: &'static str, moves: &[Move]) {
        let pos = Position::from_fen(fen).unwrap();
        let mut mov_vec = Vec::new();
        generate_moves(pos.side_to_move(), &pos, &mut mov_vec);
        let hash: HashSet<_> = mov_vec.iter().collect();
        for mov in hash {
            if !moves.contains(&mov) {
                println!("move {} was not found in collection: ", mov);
                for m in moves {
                    println!("   > {}", m);
                }

                println!("{}", pos);
                panic!()
            }
        }
    }

    fn assert_moves_contains(fen: &'static str, moves: &[Move]) {
        let pos = Position::from_fen(fen).unwrap();
        let mut mov_vec = Vec::new();
        generate_moves(pos.side_to_move(), &pos, &mut mov_vec);
        let hash: HashSet<_> = mov_vec.iter().collect();
        for mov in moves {
            if !hash.contains(mov) {
                println!("move {} was not generated", mov);
                println!("{}", pos);
                println!("moves: {:?}", mov_vec);
                panic!()
            }
        }
    }

    fn assert_moves_does_not_contain(fen: &'static str, moves: &[Move]) {
        let pos = Position::from_fen(fen).unwrap();
        let mut mov_vec = Vec::new();
        generate_moves(pos.side_to_move(), &pos, &mut mov_vec);
        let hash: HashSet<_> = mov_vec.iter().collect();
        for mov in moves {
            if hash.contains(mov) {
                println!("move list contained banned move: {}", mov);
                println!("{}", pos);
                panic!()
            }
        }
    }

    mod pawns {
        use super::*;

        #[test]
        fn white_pawn_smoke_test() {
            assert_moves_generated("8/8/8/8/5P2/8/8/8 w - - 0 1", &[Move::quiet(F4, F5)]);
        }

        #[test]
        fn white_pawn_multiple_smoke_test() {
            assert_moves_generated(
                "8/8/8/6P1/2P5/4P3/8/8 w - - 0 1",
                &[
                    Move::quiet(C4, C5),
                    Move::quiet(E3, E4),
                    Move::quiet(G5, G6),
                ],
            );
        }

        #[test]
        fn white_pawn_blocked() {
            assert_moves_generated(
                "8/8/6p1/6P1/2P1p3/4P3/8/8 w - - 0 1",
                &[Move::quiet(C4, C5)],
            );
        }

        #[test]
        fn no_pawn_push_when_target_square_occupied() {
            assert_moves_does_not_contain(
                "rnbqkbnr/1ppppppp/8/p7/P7/8/1PPPPPPP/RNBQKBNR w KQkq - 0 1",
                &[Move::quiet(A4, A5)],
            );
        }

        #[test]
        fn no_double_pawn_push_when_blocked() {
            assert_moves_does_not_contain(
                "8/8/8/8/8/4p3/4P3/8 w - - 0 1",
                &[Move::double_pawn_push(E2, E4)],
            );
        }

        #[test]
        fn double_pawn_push_smoke() {
            assert_moves_generated(
                "8/8/8/8/8/4P1p1/2P3P1/8 w - - 0 1",
                &[
                    Move::quiet(C2, C3),
                    Move::double_pawn_push(C2, C4),
                    Move::quiet(E3, E4),
                ],
            );
        }
    }
}
