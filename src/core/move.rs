// Copyright 2017-2021 Sean Gillespie.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::core::{PieceKind, Square};
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
}
