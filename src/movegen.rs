// Copyright 2021 Sean Gillespie.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
use crate::core::*;
use crate::Position;

pub fn generate_pawn_moves(us: Color, pos: &Position, moves: &mut Vec<Move>) {
    let them = us.toggle();
    let their_pieces = pos.pieces(them);
    let our_pieces = pos.pieces(us);
    let all_pieces = their_pieces.or(our_pieces);
    let empty_squares = !all_pieces;
    let (up, down, up_left, up_right, promo_rank, start_rank) = if us == Color::White {
        (
            Direction::North,
            Direction::South,
            Direction::NorthWest,
            Direction::NorthEast,
            SS_RANK_8,
            SS_RANK_2,
        )
    } else {
        (
            Direction::South,
            Direction::North,
            Direction::SouthWest,
            Direction::SouthEast,
            SS_RANK_1,
            SS_RANK_7,
        )
    };
    let rank_below_promo = promo_rank.shift(down);
    let our_pawns = pos.pawns(us);
    // Single and double pawn pushes, not counting promotions.
    {
        let single_pushes = our_pawns
            .and(!rank_below_promo)
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
            moves.push(Move::promotion(
                target.towards(up.reverse()),
                target,
                PieceKind::Bishop,
            ));
            moves.push(Move::promotion(
                target.towards(up.reverse()),
                target,
                PieceKind::Knight,
            ));
            moves.push(Move::promotion(
                target.towards(up.reverse()),
                target,
                PieceKind::Rook,
            ));
            moves.push(Move::promotion(
                target.towards(up.reverse()),
                target,
                PieceKind::Queen,
            ));
        }
    }

    // Non-promotion captures, including en-passant.
    let non_f7_pawns = our_pawns.and(!pawns_near_promo);
    {
        let up_left_cap = non_f7_pawns.shift(up_left).and(their_pieces);
        let up_right_cap = non_f7_pawns.shift(up_right).and(their_pieces);
        for target in up_left_cap {
            moves.push(Move::capture(target.towards(up_left.reverse()), target));
        }
        for target in up_right_cap {
            moves.push(Move::capture(target.towards(up_right.reverse()), target));
        }

        if let Some(ep_square) = pos.en_passant_square() {
            for source in pawn_attacks(ep_square, them).and(our_pawns) {
                moves.push(Move::en_passant(source, ep_square));
            }
        }
    }
}

pub fn generate_moves_for_kind(us: Color, pos: &Position, kind: PieceKind, moves: &mut Vec<Move>) {
    debug_assert!(
        kind != PieceKind::King && kind != PieceKind::Pawn,
        "kings and pawns have their own movegen routines"
    );

    let all_pieces = pos.pieces(Color::White) | pos.pieces(Color::Black);
    let enemy_pieces = pos.pieces(us.toggle());
    for piece in pos.pieces_of_kind(us, kind) {
        for atk in attacks(kind, us, piece, all_pieces) {
            if enemy_pieces.contains(atk) {
                moves.push(Move::capture(piece, atk));
            } else {
                moves.push(Move::quiet(piece, atk));
            }
        }
    }
}

pub fn generate_king_moves(us: Color, pos: &Position, moves: &mut Vec<Move>) {
    let enemy_pieces = pos.pieces(us.toggle());
    let allied_pieces = pos.pieces(us);
    let pieces = enemy_pieces.or(allied_pieces);
    let king = if let Some(king) = pos.king(us) {
        king
    } else {
        return;
    };
    for target in king_attacks(king) {
        if enemy_pieces.contains(target) {
            moves.push(Move::capture(king, target));
        } else if !allied_pieces.contains(target) {
            moves.push(Move::quiet(king, target));
        }
    }

    // Generate castling moves, if we are allowed to castle.
    if pos.is_check(us) {
        // No castling out of check.
        return;
    }

    if pos.can_castle_kingside(us) {
        let starting_rook = if us == Color::White { H1 } else { H8 };

        if let Some(piece) = pos.piece_at(starting_rook) {
            if piece.kind == PieceKind::Rook && piece.color == us {
                let one = king.towards(Direction::East);
                let two = one.towards(Direction::East);
                if !pieces.contains(one) && !pieces.contains(two) {
                    // The king moves across both squares one and two and it is illegal
                    // to castle through check. We can only proceed if no enemy piece is
                    // attacking the squares the king travels upon.
                    if pos.squares_attacking(us.toggle(), one).is_empty()
                        && pos.squares_attacking(us.toggle(), two).is_empty()
                    {
                        moves.push(Move::kingside_castle(king, two));
                    }
                }
            }
        }
    }

    if pos.can_castle_queenside(us) {
        let starting_rook = if us == Color::White { A1 } else { A8 };

        if let Some(piece) = pos.piece_at(starting_rook) {
            if piece.kind == PieceKind::Rook && piece.color == us {
                let one = king.towards(Direction::West);
                let two = one.towards(Direction::West);
                let three = two.towards(Direction::West);
                if !pieces.contains(one) && !pieces.contains(two) && !pieces.contains(three) {
                    // Square three can be checked, but it can't be occupied. The rook
                    // travels across square three, but the king does not.
                    if pos.squares_attacking(us.toggle(), one).is_empty()
                        && pos.squares_attacking(us.toggle(), two).is_empty()
                    {
                        moves.push(Move::queenside_castle(king, two));
                    }
                }
            }
        }
    }
}

pub fn generate_moves(us: Color, pos: &Position, moves: &mut Vec<Move>) {
    generate_pawn_moves(us, pos, moves);
    generate_moves_for_kind(us, pos, PieceKind::Bishop, moves);
    generate_moves_for_kind(us, pos, PieceKind::Knight, moves);
    generate_moves_for_kind(us, pos, PieceKind::Rook, moves);
    generate_moves_for_kind(us, pos, PieceKind::Queen, moves);
    generate_king_moves(us, pos, moves);
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
                println!("move {:?} was not found in collection: ", mov);
                for m in moves {
                    println!("   > {:?}", m);
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
        fn white_pawn_smoke_contains() {
            assert_moves_generated("8/8/8/8/5P2/8/8/8 w - - 0 1", &[Move::quiet(F4, F5)]);
        }

        #[test]
        fn white_pawn_multiple_smoke_contains() {
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

        #[test]
        fn pawn_promo_smoke() {
            assert_moves_generated(
                "8/3P4/8/8/8/8/8/8 w - - 0 1",
                &[
                    Move::promotion(D7, D8, PieceKind::Bishop),
                    Move::promotion(D7, D8, PieceKind::Knight),
                    Move::promotion(D7, D8, PieceKind::Rook),
                    Move::promotion(D7, D8, PieceKind::Queen),
                ],
            )
        }

        #[test]
        fn pawn_promo_blocked() {
            assert_moves_does_not_contain(
                "3n4/3P4/8/8/8/8/8/8 w - - 0 1",
                &[
                    Move::promotion(D7, D8, PieceKind::Bishop),
                    Move::promotion(D7, D8, PieceKind::Knight),
                    Move::promotion(D7, D8, PieceKind::Rook),
                    Move::promotion(D7, D8, PieceKind::Queen),
                ],
            )
        }

        #[test]
        fn pawn_promo_captures() {
            assert_moves_generated(
                "2nnn3/3P4/8/8/8/8/8/8 w - - 0 1",
                &[
                    Move::promotion_capture(D7, C8, PieceKind::Bishop),
                    Move::promotion_capture(D7, C8, PieceKind::Knight),
                    Move::promotion_capture(D7, C8, PieceKind::Rook),
                    Move::promotion_capture(D7, C8, PieceKind::Queen),
                    Move::promotion_capture(D7, E8, PieceKind::Bishop),
                    Move::promotion_capture(D7, E8, PieceKind::Knight),
                    Move::promotion_capture(D7, E8, PieceKind::Rook),
                    Move::promotion_capture(D7, E8, PieceKind::Queen),
                ],
            )
        }

        #[test]
        fn kiwipete_bug_1() {
            assert_moves_contains(
                "r3k2r/p1ppqpb1/bn2pnp1/3PN3/Pp2P3/2N2Q1p/1PPBBPPP/R3K2R b KQkq a3 0 1",
                &[Move::en_passant(B4, A3)],
            );
        }

        #[test]
        fn illegal_en_passant() {
            assert_moves_does_not_contain(
                "8/8/4p3/8/8/8/5P2/8 w - e7 0 1",
                &[
                    // this can happen if we are sloppy about validating the legality
                    // of EP-moves
                    Move::en_passant(F2, E7),
                ],
            );
        }
    }

    mod bishops {
        use super::*;

        #[test]
        fn smoke_contains() {
            assert_moves_generated(
                "8/8/8/8/3B4/8/8/8 w - - 0 1",
                &[
                    Move::quiet(D4, E5),
                    Move::quiet(D4, F6),
                    Move::quiet(D4, G7),
                    Move::quiet(D4, H8),
                    Move::quiet(D4, E3),
                    Move::quiet(D4, F2),
                    Move::quiet(D4, G1),
                    Move::quiet(D4, C3),
                    Move::quiet(D4, B2),
                    Move::quiet(D4, A1),
                    Move::quiet(D4, C5),
                    Move::quiet(D4, B6),
                    Move::quiet(D4, A7),
                ],
            );
        }

        #[test]
        fn smoke_capture() {
            assert_moves_generated(
                "8/8/8/2p1p3/3B4/2p1p3/8/8 w - - 0 1",
                &[
                    Move::capture(D4, E5),
                    Move::capture(D4, E3),
                    Move::capture(D4, C5),
                    Move::capture(D4, C3),
                ],
            );
        }
    }

    mod kings {
        use super::*;

        #[test]
        fn smoke_test() {
            assert_moves_generated(
                "8/8/8/8/4K3/8/8/8 w - - 0 1",
                &[
                    Move::quiet(E4, E5),
                    Move::quiet(E4, F5),
                    Move::quiet(E4, F4),
                    Move::quiet(E4, F3),
                    Move::quiet(E4, E3),
                    Move::quiet(E4, D3),
                    Move::quiet(E4, D4),
                    Move::quiet(E4, D5),
                ],
            );
        }

        #[test]
        fn position_4_check_block() {
            assert_moves_contains(
                "r2q1rk1/pP1p2pp/Q4n2/bbp1p3/Np6/1B3NBn/pPPP1PPP/R3K2R b KQ - 0 1",
                &[
                    Move::quiet(C5, C4),
                    Move::double_pawn_push(D7, D5),
                    Move::quiet(B5, C4),
                    Move::quiet(F6, D5),
                    Move::quiet(F8, F7),
                    Move::quiet(G8, H8),
                ],
            );
        }

        #[test]
        fn kingside_castle() {
            assert_moves_contains(
                "8/8/8/8/8/8/8/4K2R w K - 0 1",
                &[Move::kingside_castle(E1, G1)],
            );
        }

        #[test]
        fn queenside_castle() {
            assert_moves_contains(
                "8/8/8/8/8/8/8/R3K3 w Q - 0 1",
                &[Move::queenside_castle(E1, C1)],
            );
        }

        #[test]
        fn kingside_castle_neg() {
            assert_moves_does_not_contain(
                "8/8/8/8/8/8/8/4K2R w Q - 0 1",
                &[Move::kingside_castle(E1, G1)],
            );
        }

        #[test]
        fn queenside_castle_neg() {
            assert_moves_does_not_contain(
                "8/8/8/8/8/8/8/R3K3 w K - 0 1",
                &[Move::queenside_castle(E1, C1)],
            );
        }

        #[test]
        fn castle_through_check() {
            assert_moves_does_not_contain(
                "8/8/8/8/5r2/8/8/4K2R w - - 0 1",
                &[Move::kingside_castle(E1, G1)],
            );
        }

        #[test]
        fn kingside_castle_when_space_occupied() {
            assert_moves_does_not_contain(
                "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
                &[Move::kingside_castle(E1, G1)],
            );
        }

        #[test]
        fn queenside_castle_when_space_occupied() {
            assert_moves_does_not_contain(
                "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
                &[Move::queenside_castle(E1, C1)],
            );
        }

        #[test]
        fn kiwipete_bug_2() {
            assert_moves_contains(
                "r3k2r/p1pNqpb1/bn2pnp1/3P4/1p2P3/2N2Q1p/PPPBBPPP/R3K2R b KQkq - 0 1",
                &[Move::queenside_castle(E8, C8)],
            );
        }

        #[test]
        fn kiwipete_bug_3() {
            assert_moves_does_not_contain(
                "2kr3r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/5Q1p/PPPBBPPP/RN2K2R w KQ - 2 2",
                &[
                    // there's a knight on b1, this blocks castling even though it
                    // doesn't block the king's movement
                    Move::queenside_castle(E1, C1),
                ],
            )
        }
    }
}
