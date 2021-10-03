// Copyright 2017-2021 Sean Gillespie.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::{core::*, Position};
use std::convert::TryFrom;
use std::fmt::{self, Write};

const SOURCE_MASK: u16 = 0xFC00;
const DESTINATION_MASK: u16 = 0x03F0;
const PROMO_BIT: u16 = 0x0008;
const CAPTURE_BIT: u16 = 0x0004;
const SPECIAL_0_BIT: u16 = 0x0002;
const SPECIAL_1_BIT: u16 = 0x0001;
const ATTR_MASK: u16 = 0x000F;

/// A move, recognized by the a4 engine. It is designed to be as
/// compact as possible.
/// ## Encoding
/// The encoding of a move is done via this breakdown:
///
///  * 6 bits - source square
///  * 6 bits - destination square
///  * 1 bit  - promotion bit
///  * 1 bit  - capture bit
///  * 1 bit  - "special 0" bit
///  * 1 bit  - "special 1" bit
///
/// The "special" bits are overloaded, because chess has a
/// number of "special" moves that do not fit nicely into
/// a compact representation. Here is a full table of
/// the encoding strategy:
///
/// | Promo | Capt  | Spc 0 | Spc 1 | Move                   |
/// |-------|-------|-------|-------|------------------------|
/// | 0     | 0     | 0     | 0     | Quiet                  |
/// | 0     | 0     | 0     | 1     | Double Pawn            |
/// | 0     | 0     | 1     | 0     | King Castle            |
/// | 0     | 0     | 1     | 1     | Queen Castle           |
/// | 0     | 1     | 0     | 0     | Capture                |
/// | 0     | 1     | 0     | 1     | En Passant Capture     |
/// | 1     | 0     | 0     | 0     | Knight Promote         |
/// | 1     | 0     | 0     | 1     | Bishop Promote         |
/// | 1     | 0     | 1     | 0     | Rook Promote           |
/// | 1     | 0     | 1     | 1     | Queen Promote          |
/// | 1     | 1     | 0     | 0     | Knight Promote Capture |
/// | 1     | 1     | 0     | 1     | Bishop Promote Capture |
/// | 1     | 1     | 1     | 0     | Rook Promote Capture   |
/// | 1     | 1     | 1     | 1     | Queen Promote Capture  |
///
/// Thanks to [this ChessProgramming Wiki page](https://chessprogramming.wikispaces.com/Encoding+Moves)
/// for the details.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct Move(u16);

impl Move {
    /// Constructs a new quiet move from the source square to the destination
    /// square.
    pub fn quiet(source: Square, dest: Square) -> Move {
        let source_bits = (source.0 as u16) << 10;
        let dest_bits = (dest.0 as u16) << 4;
        Move(source_bits | dest_bits)
    }

    /// Constructs a new capture move from the source square to the destination
    /// square.
    pub fn capture(source: Square, dest: Square) -> Move {
        let mut mov = Move::quiet(source, dest);
        mov.0 |= CAPTURE_BIT;
        mov
    }

    /// Constructs a new en passsant move from the source square to the
    /// destination square.
    pub fn en_passant(source: Square, dest: Square) -> Move {
        let mut mov = Move::capture(source, dest);
        mov.0 |= SPECIAL_1_BIT;
        mov
    }

    /// Constructs a new double pawn push move from the source square to
    /// the destination square.
    pub fn double_pawn_push(source: Square, dest: Square) -> Move {
        let mut mov = Move::quiet(source, dest);
        mov.0 |= SPECIAL_1_BIT;
        mov
    }

    /// Constructs a new capture move from the source square to the destination
    /// square, promoting the current piece to the given piece kind.
    pub fn promotion(source: Square, dest: Square, promoted: PieceKind) -> Move {
        let mut mov = Move::quiet(source, dest);
        mov.0 |= PROMO_BIT;
        match promoted {
            PieceKind::Knight => mov.0 |= 0,
            PieceKind::Bishop => mov.0 |= 1,
            PieceKind::Rook => mov.0 |= 2,
            PieceKind::Queen => mov.0 |= 3,
            _ => panic!("invalid promotion piece"),
        }

        mov
    }

    /// Constructs a new promotion capture move from the source square to the
    /// destination square, promoting the current piece to the given piece kind.
    pub fn promotion_capture(source: Square, dest: Square, promotion: PieceKind) -> Move {
        let mut mov = Move::promotion(source, dest, promotion);
        mov.0 |= CAPTURE_BIT;
        mov
    }

    /// Constructs a new kingside castle from the source square to the
    /// destination square.
    pub fn kingside_castle(source: Square, dest: Square) -> Move {
        let mut mov = Move::quiet(source, dest);
        mov.0 |= SPECIAL_0_BIT;
        mov
    }

    /// Constructs a new queenside castle from the source square to the
    /// destination square.
    pub fn queenside_castle(source: Square, dest: Square) -> Move {
        let mut mov = Move::quiet(source, dest);
        mov.0 |= SPECIAL_0_BIT | SPECIAL_1_BIT;
        mov
    }

    /// Constructs a null move; a move that does nothing.
    pub fn null() -> Move {
        Move(0)
    }

    /// If this move is a promotion, returns the piece kind that the
    /// pawn is being promoted to. Panics if the move is not a promotion.
    pub fn promotion_piece(self) -> PieceKind {
        assert!(self.is_promotion());
        let piece = self.0 & (SPECIAL_0_BIT | SPECIAL_1_BIT);
        match piece {
            0 => PieceKind::Knight,
            1 => PieceKind::Bishop,
            2 => PieceKind::Rook,
            3 => PieceKind::Queen,
            _ => unreachable!(),
        }
    }

    /// Returns the source square of this move.
    pub fn source(self) -> Square {
        Square(((self.0 & SOURCE_MASK) >> 10) as u8)
    }

    /// Returns the destination square of this move.
    pub fn destination(self) -> Square {
        Square(((self.0 & DESTINATION_MASK) >> 4) as u8)
    }

    /// Returns whether or not this move is a quiet move.
    pub fn is_quiet(self) -> bool {
        (self.0 & ATTR_MASK) == 0
    }

    /// Returns whether or not this move is a capture move.
    pub fn is_capture(self) -> bool {
        (self.0 & CAPTURE_BIT) != 0
    }

    /// Returns whether or not this move is an en passant move.
    pub fn is_en_passant(self) -> bool {
        (self.0 & ATTR_MASK) == 5
    }

    /// Returns whether or not this move is a double pawn push.
    pub fn is_double_pawn_push(self) -> bool {
        (self.0 & ATTR_MASK) == 1
    }

    /// Returns whether or not this move is a promotion.
    pub fn is_promotion(self) -> bool {
        (self.0 & PROMO_BIT) != 0
    }

    /// Returns whether or not this move is a kingside castle.
    pub fn is_kingside_castle(self) -> bool {
        (self.0 & ATTR_MASK) == 2
    }

    /// Returns whether or not this move is a queenside castle.
    pub fn is_queenside_castle(self) -> bool {
        (self.0 & ATTR_MASK) == 3
    }

    /// Returns whether or not this move is a castle.
    pub fn is_castle(self) -> bool {
        self.is_kingside_castle() || self.is_queenside_castle()
    }

    /// Returns whether or not this move is a null move.
    pub fn is_null(self) -> bool {
        self.0 == 0
    }

    /// Returns an UCI-compatible string representation of this move.
    pub fn as_uci(self) -> String {
        // Quick out for this weird quirk of UCI: the null move is 0000.
        if self.is_null() {
            return "0000".to_string();
        }

        let mut buf = String::new();
        if !self.is_promotion() {
            write!(&mut buf, "{}{}", self.source(), self.destination()).unwrap();
        } else {
            write!(
                &mut buf,
                "{}{}{}",
                self.source(),
                self.destination(),
                self.promotion_piece()
            )
            .unwrap();
        }

        buf
    }

    /// Parses the UCI representation of a move into a Move.
    pub fn from_uci(pos: &Position, move_str: &str) -> Option<Move> {
        // UCI encodes a move as the source square, followed by the destination
        // square, and optionally followed by the promotion piece if necessary.
        let move_chrs: Vec<_> = move_str.chars().collect();
        if move_chrs.len() < 4 {
            // It's not a valid move encoding at all if it's this short.
            return None;
        }

        // A particular quirk of UCI is that null moves are encoded as 0000.
        if move_str == "0000" {
            return Some(Move::null());
        }

        let source_file = File::try_from(move_chrs[0]).ok()?;
        let source_rank = Rank::try_from(move_chrs[1]).ok()?;
        let dest_file = File::try_from(move_chrs[2]).ok()?;
        let dest_rank = Rank::try_from(move_chrs[3]).ok()?;
        let maybe_promotion_piece = if move_chrs.len() == 5 {
            Some(move_chrs[4])
        } else {
            None
        };

        let source = Square::of(source_rank, source_file);
        let dest = Square::of(dest_rank, dest_file);

        // This method is annoyingly complex, so read this here first!
        //
        // We're going to assume that a move is quiet if it's not any other category
        // of move. This means that we might not produce a legal move, but it's up
        // to the legality tests later on to make sure that this move is legit.
        //
        // There are a bunch of cases here that we have to handle. They are encoded
        // in this decision tree:
        // 1. Is the moving piece a pawn?
        //   1.1. Is the moving square two squares straight ahead? => DoublePawnPush
        //   1.2. Is the moving square a legal attack for a pawn?
        //     1.2.1. Is the destination square on a promotion rank? =>
        //     PromotionCapture
        //     1.2.2. Is the destination square the en-passant square?
        //     => EnPassant
        //     1.2.3. else => Capture
        //   1.3. Is the destination square on a promotion rank? =? Promotion
        //   1.4. else => Quiet
        // 2. Is the moving piece a king?
        //   2.1. Is the target the square to the right of the kingside rook? =>
        //   KingsideCastle
        //   2.2. Is the target the square to the right of the queenside rook? =>
        //   QueensideCastle
        //   2.3. Is there piece on the target square? => Capture
        //   2.4. else => Quiet
        // 3. Is there a piece on the target square? => Capture
        // 4. else => Quiet
        //
        // Whew!
        let dest_piece = pos.piece_at(dest);
        let moving_piece = pos.piece_at(source)?;

        // 1. Is the moving piece a pawn?
        if moving_piece.kind == PieceKind::Pawn {
            let (pawn_dir, promo_rank, start_rank) = match pos.side_to_move() {
                Color::White => (Direction::North, SS_RANK_8, SS_RANK_2),
                Color::Black => (Direction::South, SS_RANK_1, SS_RANK_7),
            };

            // 1.1. Is the moving square two squares straight ahead?
            if start_rank.contains(source) {
                let double_pawn_square = source.towards(pawn_dir).towards(pawn_dir);
                if double_pawn_square == dest {
                    return Some(Move::double_pawn_push(source, dest));
                }
            }

            // 1.2. Is the moving square a legal attack for a pawn?
            if attacks::pawn_attacks(source, pos.side_to_move()).contains(dest) {
                // 1.2.1. Is the destination square on a promotion rank?
                if promo_rank.contains(dest) {
                    let promo_piece = maybe_promotion_piece?;
                    let kind = match promo_piece {
                        'n' => PieceKind::Knight,
                        'b' => PieceKind::Bishop,
                        'r' => PieceKind::Rook,
                        'q' => PieceKind::Queen,
                        _ => return None,
                    };

                    return Some(Move::promotion_capture(source, dest, kind));
                }

                // 1.2.2. Is the destination square the en-passant square?
                if Some(dest) == pos.en_passant_square() {
                    return Some(Move::en_passant(source, dest));
                }

                // 1.2.3. Else, it's a capture.
                return Some(Move::capture(source, dest));
            }

            // 1.3. Is the destination square on a promotion rank?
            if promo_rank.contains(dest) {
                let promo_piece = maybe_promotion_piece?;
                let kind = match promo_piece {
                    'n' => PieceKind::Knight,
                    'b' => PieceKind::Bishop,
                    'r' => PieceKind::Rook,
                    'q' => PieceKind::Queen,
                    _ => return None,
                };

                return Some(Move::promotion(source, dest, kind));
            }

            // 1.4. Else, it's a quiet move.
            return Some(Move::quiet(source, dest));
        }

        // 2. Is the moving piece a king?
        if moving_piece.kind == PieceKind::King {
            let (kingside_rook_adjacent, queenside_rook_adjacent, king_start) =
                match pos.side_to_move() {
                    Color::White => (G1, C1, E1),
                    Color::Black => (G8, C8, E8),
                };

            if king_start == source {
                // 2.1. Is the target of the square to the left of the kingside rook?
                if kingside_rook_adjacent == dest {
                    return Some(Move::kingside_castle(source, dest));
                }

                // 2.2. Is the target the square to the right of the queenside rook?
                if queenside_rook_adjacent == dest {
                    return Some(Move::queenside_castle(source, dest));
                }
            }

            // 2.3. Is there a piece on the target square?
            if dest_piece.is_some() {
                return Some(Move::capture(source, dest));
            }

            // 2.4. Else, it's quiet.
            return Some(Move::quiet(source, dest));
        }

        // 3. Is there a piece on the target square?
        if dest_piece.is_some() {
            return Some(Move::capture(source, dest));
        }

        // 4. Else, it's quiet.
        return Some(Move::quiet(source, dest));
    }
}

impl fmt::Display for Move {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", self.as_uci())
    }
}

impl fmt::Debug for Move {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{} (0x{:x})", self.as_uci(), self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::Move;
    use crate::core::*;
    use crate::Position;

    #[test]
    fn quiet() {
        let quiet = Move::quiet(A4, A5);
        assert_eq!(A4, quiet.source());
        assert_eq!(A5, quiet.destination());
        assert!(quiet.is_quiet());
    }

    #[test]
    fn capture() {
        let capture = Move::capture(B4, C4);
        assert_eq!(B4, capture.source());
        assert_eq!(C4, capture.destination());
        assert!(!capture.is_quiet());
        assert!(capture.is_capture());
    }

    #[test]
    fn en_passant() {
        let ep = Move::en_passant(A1, B2);
        assert!(ep.is_en_passant());
        assert!(ep.is_capture());
        assert!(!ep.is_quiet());
    }

    #[test]
    fn double_pawn_push() {
        let dpp = Move::double_pawn_push(D2, D4);
        assert!(dpp.is_double_pawn_push());
        assert!(!dpp.is_capture());
        assert!(!dpp.is_quiet());
    }

    fn promotion(kind: PieceKind) {
        let promo = Move::promotion(A2, H2, kind);
        assert!(promo.is_promotion());
        assert!(!promo.is_capture());
        assert_eq!(kind, promo.promotion_piece());
    }

    fn promo_capture(kind: PieceKind) {
        let promo = Move::promotion_capture(B2, C2, kind);
        assert!(promo.is_promotion());
        assert!(promo.is_capture());
        assert_eq!(kind, promo.promotion_piece());
    }

    #[test]
    fn promotion_bishop() {
        promotion(PieceKind::Bishop)
    }

    #[test]
    fn promotion_knight() {
        promotion(PieceKind::Knight)
    }

    #[test]
    fn promotion_rook() {
        promotion(PieceKind::Rook)
    }

    #[test]
    fn promotion_queen() {
        promotion(PieceKind::Queen)
    }

    #[test]
    fn promotion_capture_bishop() {
        promo_capture(PieceKind::Bishop)
    }

    #[test]
    fn promotion_capture_knight() {
        promo_capture(PieceKind::Knight)
    }

    #[test]
    fn promotion_capture_rook() {
        promo_capture(PieceKind::Rook)
    }

    #[test]
    fn promotion_capture_queen() {
        promo_capture(PieceKind::Queen)
    }

    #[test]
    fn kingside_castle() {
        let mv = Move::kingside_castle(A1, A2);
        assert!(mv.is_kingside_castle());
        assert!(!mv.is_queenside_castle());
        assert!(!mv.is_capture());
    }

    #[test]
    fn queenside_castle() {
        let mv = Move::queenside_castle(A2, H2);
        assert!(mv.is_queenside_castle());
        assert!(!mv.is_kingside_castle());
        assert!(!mv.is_capture());
    }

    #[test]
    fn uci_null() {
        let mv = Move::null();
        assert_eq!("0000", mv.as_uci());
    }

    #[test]
    fn uci_smoke() {
        let mv = Move::quiet(A1, A2);
        assert_eq!("a1a2", mv.as_uci());
    }

    #[test]
    fn uci_promote() {
        let mv = Move::promotion(A7, A8, PieceKind::Queen);
        assert_eq!("a7a8q", mv.as_uci());
    }

    #[test]
    fn uci_kingside_castle() {
        let mv = Move::kingside_castle(E1, G1);
        assert_eq!("e1g1", mv.as_uci());
    }

    #[test]
    fn uci_nullmove() {
        let pos = Position::from_start_position();
        assert_eq!(Move::null(), Move::from_uci(&pos, "0000").unwrap());
    }

    #[test]
    fn uci_sliding_moves() {
        let pos = Position::from_fen("8/3q4/8/8/8/3R4/8/8 w - - 0 1").unwrap();
        assert_eq!(Move::quiet(D3, D5), Move::from_uci(&pos, "d3d5").unwrap());
        assert_eq!(Move::capture(D3, D7), Move::from_uci(&pos, "d3d7").unwrap());
    }

    #[test]
    fn uci_pawn_moves() {
        let pos = Position::from_fen("8/8/8/8/8/4p3/3P4/8 w - c3 0 1").unwrap();
        assert_eq!(Move::quiet(D2, D3), Move::from_uci(&pos, "d2d3").unwrap());
        assert_eq!(
            Move::double_pawn_push(D2, D4),
            Move::from_uci(&pos, "d2d4").unwrap()
        );
        assert_eq!(Move::capture(D2, E3), Move::from_uci(&pos, "d2e3").unwrap());
        assert_eq!(Move::quiet(D2, D3), Move::from_uci(&pos, "d2d3").unwrap());
        assert_eq!(
            Move::en_passant(D2, C3),
            Move::from_uci(&pos, "d2c3").unwrap()
        );
    }

    #[test]
    fn uci_king_moves() {
        let pos = Position::from_fen("8/8/8/8/8/8/3r4/R3K2R w - - 0 1").unwrap();
        assert_eq!(
            Move::kingside_castle(E1, G1),
            Move::from_uci(&pos, "e1g1").unwrap(),
        );
        assert_eq!(
            Move::queenside_castle(E1, C1),
            Move::from_uci(&pos, "e1c1").unwrap(),
        );
        assert_eq!(Move::quiet(E1, E2), Move::from_uci(&pos, "e1e2").unwrap(),);
        assert_eq!(Move::capture(E1, D2), Move::from_uci(&pos, "e1d2").unwrap(),);
    }

    #[test]
    fn uci_promotion() {
        let pos = Position::from_fen("5n2/4P3/8/8/8/8/8/8 w - - 0 1").unwrap();
        assert_eq!(
            Move::promotion(E7, E8, PieceKind::Knight),
            Move::from_uci(&pos, "e7e8n").unwrap()
        );
        assert_eq!(
            Move::promotion(E7, E8, PieceKind::Bishop),
            Move::from_uci(&pos, "e7e8b").unwrap()
        );
        assert_eq!(
            Move::promotion(E7, E8, PieceKind::Rook),
            Move::from_uci(&pos, "e7e8r").unwrap()
        );
        assert_eq!(
            Move::promotion(E7, E8, PieceKind::Queen),
            Move::from_uci(&pos, "e7e8q").unwrap()
        );
        assert_eq!(
            Move::promotion_capture(E7, F8, PieceKind::Knight),
            Move::from_uci(&pos, "e7f8n").unwrap()
        );
        assert_eq!(
            Move::promotion_capture(E7, F8, PieceKind::Bishop),
            Move::from_uci(&pos, "e7f8b").unwrap()
        );
        assert_eq!(
            Move::promotion_capture(E7, F8, PieceKind::Rook),
            Move::from_uci(&pos, "e7f8r").unwrap()
        );
        assert_eq!(
            Move::promotion_capture(E7, F8, PieceKind::Queen),
            Move::from_uci(&pos, "e7f8q").unwrap()
        );
    }
}
