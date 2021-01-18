// Copyright 2017-2021 Sean Gillespie.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::core::{
    self, CastleStatus, Color, File, Move, Piece, PieceKind, Rank, Square, SquareSet,
};
use std::convert::TryFrom;
use std::fmt::{self, Write};
use thiserror::Error;

/// Information that can't be recovered normally when unmaking a move. When making or unmaking a move, this information
/// is cloned instead of incrementally updated.
///
/// Because this structure is cloned, care must be taken to make this structure as small as possible.
#[derive(Clone, Debug)]
struct IrreversibleInformation {
    mov: Option<Move>,
    halfmove_clock: u16,
    fullmove_clock: u16,
    castle_status: CastleStatus,
    en_passant_square: Option<Square>,
}

/// A position, representing a chess game that has progressed up to this point. A Position encodes the complete state
/// of the game such that the entire game up until this point can be recovered and reconstructed efficiently.
///
/// The primary mutable operations on a position are `make` and `unmake`, which apply and un-apply a move to the board
/// state, respectively.
///
/// Almost everything about `Position` is performance-critical.
#[derive(Clone, Debug)]
pub struct Position {
    /// SquareSets for each piece and color combination (6 pieces, 2 colors = 12 sets).
    sets_by_piece: [SquareSet; 12],

    /// Squaresets for each color.
    sets_by_color: [SquareSet; 2],

    /// The set of irreversible information for the current position.
    current_information: IrreversibleInformation,

    /// The list of irreversible informations for all previous positions in this game. Using this, the complete history
    /// of the game can be recovered.
    previous_information: Vec<IrreversibleInformation>,

    /// Color whose turn it is to move.
    side_to_move: Color,
}

impl Position {
    pub fn en_passant_square(&self) -> Option<Square> {
        self.current_information.en_passant_square
    }

    pub fn halfmove_clock(&self) -> u16 {
        self.current_information.halfmove_clock
    }

    pub fn fullmove_clock(&self) -> u16 {
        self.current_information.fullmove_clock
    }

    pub fn side_to_move(&self) -> Color {
        self.side_to_move
    }

    pub fn can_castle_kingside(&self, color: Color) -> bool {
        match color {
            Color::White => self
                .current_information
                .castle_status
                .contains(CastleStatus::WHITE_KINGSIDE),
            Color::Black => self
                .current_information
                .castle_status
                .contains(CastleStatus::BLACK_KINGSIDE),
        }
    }

    pub fn can_castle_queenside(&self, color: Color) -> bool {
        match color {
            Color::White => self
                .current_information
                .castle_status
                .contains(CastleStatus::WHITE_QUEENSIDE),
            Color::Black => self
                .current_information
                .castle_status
                .contains(CastleStatus::BLACK_QUEENSIDE),
        }
    }

    pub fn pieces(&self, color: Color) -> SquareSet {
        self.sets_by_color[color as usize]
    }

    pub fn pieces_of_kind(&self, color: Color, kind: PieceKind) -> SquareSet {
        let offset = match color {
            Color::White => 0,
            Color::Black => 6,
        };
        self.sets_by_piece[offset + kind as usize]
    }

    pub fn pawns(&self, color: Color) -> SquareSet {
        self.pieces_of_kind(color, PieceKind::Pawn)
    }

    pub fn bishops(&self, color: Color) -> SquareSet {
        self.pieces_of_kind(color, PieceKind::Bishop)
    }

    pub fn knights(&self, color: Color) -> SquareSet {
        self.pieces_of_kind(color, PieceKind::Knight)
    }

    pub fn rooks(&self, color: Color) -> SquareSet {
        self.pieces_of_kind(color, PieceKind::Rook)
    }

    pub fn queens(&self, color: Color) -> SquareSet {
        self.pieces_of_kind(color, PieceKind::Queen)
    }

    pub fn kings(&self, color: Color) -> SquareSet {
        self.pieces_of_kind(color, PieceKind::King)
    }
}

impl Position {
    pub fn new() -> Position {
        Position {
            sets_by_piece: [SquareSet::empty(); 12],
            sets_by_color: [SquareSet::empty(); 2],
            current_information: IrreversibleInformation {
                mov: None,
                halfmove_clock: 0,
                fullmove_clock: 0,
                castle_status: CastleStatus::BLACK | CastleStatus::WHITE,
                en_passant_square: None,
            },
            previous_information: vec![],
            side_to_move: Color::White,
        }
    }

    pub fn add_piece(&mut self, square: Square, piece: Piece) -> Result<(), ()> {
        if self.piece_at(square).is_some() {
            return Err(());
        }

        self.sets_by_color[piece.color as usize].insert(square);
        let offset = if piece.color == Color::White { 0 } else { 6 };
        self.sets_by_piece[piece.kind as usize + offset].insert(square);
        Ok(())
    }

    pub fn remove_piece(&mut self, square: Square) -> Result<(), ()> {
        let existing_piece = if let Some(piece) = self.piece_at(square) {
            piece
        } else {
            return Err(());
        };

        self.sets_by_color[existing_piece.color as usize].remove(square);
        let offset = if existing_piece.color == Color::White {
            0
        } else {
            6
        };
        self.sets_by_piece[existing_piece.kind as usize + offset].remove(square);
        Ok(())
    }

    pub fn piece_at(&self, square: Square) -> Option<Piece> {
        let (board_offset, color) = if self.sets_by_color[Color::White as usize].contains(square) {
            (0, Color::White)
        } else if self.sets_by_color[Color::Black as usize].contains(square) {
            (6, Color::Black)
        } else {
            return None;
        };

        for kind in core::piece_kinds() {
            let board = self.sets_by_piece[kind as usize + board_offset];
            if board.contains(square) {
                return Some(Piece { kind, color });
            }
        }

        // If we get here, we failed to update a bitboard somewhere.
        unreachable!()
    }
}

//
// Make and unmake move and associated state update functions.
//

impl Position {
    /// Makes a move on the position, updating all internal state to reflect the effects of the move.
    pub fn make_move(&mut self, mov: Move) {
        // Position works by incrementally updating the lion's share of state while cloning and persisting the things
        // that can't be incrementally updated.
        //
        // First thing we do: stash away this move's irreversible state. From here on out we will be destructively
        // modifiying all state in the position.
        self.current_information.mov = Some(mov);
        self.previous_information
            .push(self.current_information.clone());

        if mov.is_null() {
            // Quick out for null moves:
            //  1. EP is not legal next turn.
            //  2. Halfmove clock always increases.
            //  3. Fullmove clock increases if Black is the one making the null move.
            self.current_information.en_passant_square = None;
            self.current_information.halfmove_clock += 1;
            self.side_to_move = self.side_to_move.toggle();
            if self.side_to_move == Color::White {
                self.current_information.fullmove_clock += 1;
            }

            return;
        }

        let moving_piece = self
            .piece_at(mov.source())
            .expect("no piece at move source");
        self.remove_piece(mov.source())
            .expect("no piece at move source");
        self.add_piece(mov.destination(), moving_piece).expect(
            "piece present at move destination, should have been removed already if capture",
        );
        self.side_to_move = self.side_to_move.toggle();
        if self.side_to_move == Color::White {
            self.current_information.fullmove_clock += 1;
        }
    }

    pub fn unmake_move(&mut self, mov: Move) {
        // Almost all of our state will be re-calculated by incrementally unmaking the given move on our current state.
        // The bits that can't be inferred from our current state are stored in the `previous_information` vector,
        // which we'll pop and slam into `current_information`.
        //
        // First, we do two checks to make sure that what we're doing isn't totally invalid:
        // 1. There needs to be at least one move to unmake
        assert!(!self.previous_information.is_empty());
        // 2. The move we're unmaking needs to be the most recently applied move.
        assert!(self.previous_information.last().unwrap().mov.unwrap() == mov);

        // Restore the previous move's information into our current information slot.
        self.current_information = self.previous_information.pop().unwrap();
        // Clear out the move; we're unmaking it and we'll overwrite it later when we make another move.
        self.current_information.mov = None;

        // The rest of unmake_move proceeds as the reverse of make_move, for the most part.
        let moved_piece = self
            .piece_at(mov.destination())
            .expect("no piece at move destination");
        self.remove_piece(mov.destination())
            .expect("no piece at move destination");
        self.add_piece(mov.source(), moved_piece)
            .expect("piece at move source");
        self.side_to_move = self.side_to_move.toggle();
    }
}

//
// FEN and UCI parsing and generation.
//
// The routines in this block are oriented around FEN, a simple notation for chess positions.
// Positions can be created by parsing FEN and FEN can be produced from particular positions.
//
// UCI move parsing is also done here. It is not necessarily straightforward to derive a Move
// representation from a UCI move string; it requires full knowledge of the current position to
// disambiguate a move.
//

/// Possible errors that can arise when parsing a FEN string into a `Position`.
#[derive(Clone, PartialEq, Eq, Debug, Error)]
pub enum FenParseError {
    #[error("unexpected char: {0}")]
    UnexpectedChar(char),
    #[error("unexpected EOF while reading")]
    UnexpectedEnd,
    #[error("invalid digit")]
    InvalidDigit,
    #[error("file does not sum to 8")]
    FileDoesNotSumToEight,
    #[error("unknown piece: {0}")]
    UnknownPiece(char),
    #[error("invalid side to move")]
    InvalidSideToMove,
    #[error("invalid castle")]
    InvalidCastle,
    #[error("invalid en-passant")]
    InvalidEnPassant,
    #[error("empty halfmove")]
    EmptyHalfmove,
    #[error("invalid halfmove")]
    InvalidHalfmove,
    #[error("empty fullmove")]
    EmptyFullmove,
    #[error("invalid fullmove")]
    InvalidFullmove,
}

impl Position {
    pub fn from_start_position() -> Position {
        Position::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap()
    }

    /// Constructs a new position from a FEN representation of a board position.
    pub fn from_fen(fen: impl AsRef<str>) -> Result<Position, FenParseError> {
        use std::iter::Peekable;
        use std::str::Chars;

        type Stream<'a> = Peekable<Chars<'a>>;

        fn eat<'a>(iter: &mut Stream<'a>, expected: char) -> Result<(), FenParseError> {
            match iter.next() {
                Some(c) if c == expected => Ok(()),
                Some(c) => Err(FenParseError::UnexpectedChar(c)),
                None => Err(FenParseError::UnexpectedEnd),
            }
        }

        fn advance<'a>(iter: &mut Stream<'a>) -> Result<(), FenParseError> {
            let _ = iter.next();
            Ok(())
        }

        fn peek<'a>(iter: &mut Stream<'a>) -> Result<char, FenParseError> {
            if let Some(c) = iter.peek() {
                Ok(*c)
            } else {
                Err(FenParseError::UnexpectedEnd)
            }
        }

        fn eat_side_to_move<'a>(iter: &mut Stream<'a>) -> Result<Color, FenParseError> {
            let side = match peek(iter)? {
                'w' => Color::White,
                'b' => Color::Black,
                _ => return Err(FenParseError::InvalidSideToMove),
            };

            advance(iter)?;
            Ok(side)
        }

        fn eat_castle_status<'a>(iter: &mut Stream<'a>) -> Result<CastleStatus, FenParseError> {
            if peek(iter)? == '-' {
                advance(iter)?;
                return Ok(CastleStatus::NONE);
            }

            let mut status = CastleStatus::NONE;
            for _ in 0..4 {
                match peek(iter)? {
                    'K' => status |= CastleStatus::WHITE_KINGSIDE,
                    'k' => status |= CastleStatus::BLACK_KINGSIDE,
                    'Q' => status |= CastleStatus::WHITE_QUEENSIDE,
                    'q' => status |= CastleStatus::BLACK_QUEENSIDE,
                    ' ' => break,
                    _ => return Err(FenParseError::InvalidCastle),
                }

                advance(iter)?;
            }

            Ok(status)
        }

        fn eat_en_passant<'a>(iter: &mut Stream<'a>) -> Result<Option<Square>, FenParseError> {
            let c = peek(iter)?;
            if c == '-' {
                advance(iter)?;
                return Ok(None);
            }

            if let Ok(file) = File::try_from(c) {
                advance(iter)?;
                let rank_c = peek(iter)?;
                if let Ok(rank) = Rank::try_from(rank_c) {
                    advance(iter)?;
                    Ok(Some(Square::of(rank, file)))
                } else {
                    Err(FenParseError::InvalidEnPassant)
                }
            } else {
                Err(FenParseError::InvalidEnPassant)
            }
        }

        fn eat_halfmove<'a>(iter: &mut Stream<'a>) -> Result<u16, FenParseError> {
            let mut buf = String::new();
            loop {
                let c = peek(iter)?;
                if !c.is_digit(10) {
                    break;
                }

                buf.push(c);
                advance(iter)?;
            }

            if buf.is_empty() {
                return Err(FenParseError::EmptyHalfmove);
            }

            buf.parse::<u16>()
                .map_err(|_| FenParseError::InvalidHalfmove)
        }

        fn eat_fullmove<'a>(iter: &mut Stream<'a>) -> Result<u16, FenParseError> {
            let mut buf = String::new();
            for ch in iter {
                if !ch.is_digit(10) {
                    if buf.is_empty() {
                        return Err(FenParseError::EmptyFullmove);
                    }

                    break;
                }

                buf.push(ch);
            }

            if buf.is_empty() {
                return Err(FenParseError::EmptyFullmove);
            }

            buf.parse::<u16>()
                .map_err(|_| FenParseError::InvalidFullmove)
        }

        let mut pos = Position::new();
        let str_ref = fen.as_ref();
        let iter = &mut str_ref.chars().peekable();
        for rank in core::ranks().rev() {
            let mut file = 0;
            while file <= 7 {
                let c = peek(iter)?;
                // digits 1 through 8 indicate empty squares.
                if c.is_digit(10) {
                    if c < '1' || c > '8' {
                        return Err(FenParseError::InvalidDigit);
                    }

                    let value = c as usize - 48;
                    file += value;
                    if file > 8 {
                        return Err(FenParseError::FileDoesNotSumToEight);
                    }

                    advance(iter)?;
                    continue;
                }

                // if it's not a digit, it represents a piece.
                let piece = if let Ok(piece) = Piece::try_from(c) {
                    piece
                } else {
                    return Err(FenParseError::UnknownPiece(c));
                };

                let square = Square::of(rank, File::try_from(file as u8).unwrap());
                pos.add_piece(square, piece).expect("FEN double-add piece?");
                advance(iter)?;
                file += 1;
            }

            if rank != core::RANK_1 {
                eat(iter, '/')?;
            }
        }

        eat(iter, ' ')?;
        pos.side_to_move = eat_side_to_move(iter)?;
        eat(iter, ' ')?;
        pos.current_information.castle_status = eat_castle_status(iter)?;
        eat(iter, ' ')?;
        pos.current_information.en_passant_square = eat_en_passant(iter)?;
        eat(iter, ' ')?;
        pos.current_information.halfmove_clock = eat_halfmove(iter)?;
        eat(iter, ' ')?;
        pos.current_information.fullmove_clock = eat_fullmove(iter)?;
        Ok(pos)
    }

    /*
    /// Parses the UCI representation of a move into a Move object, suitable as an argument to
    /// `apply_move`.
    pub fn move_from_uci(&self, move_str: &str) -> Option<Move> {
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
        let dest_piece = self.piece_at(dest);
        let moving_piece = self.piece_at(source)?;

        // 1. Is the moving piece a pawn?
        if moving_piece.kind == PieceKind::Pawn {
            let (pawn_dir, promo_rank, start_rank) = match self.side_to_move {
                Color::White => (Direction::North, BB_RANK_8, BB_RANK_2),
                Color::Black => (Direction::South, BB_RANK_1, BB_RANK_7),
            };

            // 1.1. Is the moving square two squares straight ahead?
            if start_rank.test(source) {
                let double_pawn_square = source.towards(pawn_dir).towards(pawn_dir);
                if double_pawn_square == dest {
                    return Some(Move::double_pawn_push(source, dest));
                }
            }

            // 1.2. Is the moving square a legal attack for a pawn?
            if attacks::pawn_attacks(source, self.side_to_move).test(dest) {
                // 1.2.1. Is the destination square on a promotion rank?
                if promo_rank.test(dest) {
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
                if Some(dest) == self.en_passant_square {
                    return Some(Move::en_passant(source, dest));
                }

                // 1.2.3. Else, it's a capture.
                return Some(Move::capture(source, dest));
            }

            // 1.3. Is the destination square on a promotion rank?
            if promo_rank.test(dest) {
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
                match self.side_to_move {
                    Color::White => (Square::G1, Square::C1, Square::E1),
                    Color::Black => (Square::G8, Square::C8, Square::E8),
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
    */

    pub fn as_fen(&self) -> String {
        let mut buf = String::new();
        for rank in core::ranks().rev() {
            let mut empty_squares = 0;
            for file in core::files() {
                let square = Square::of(rank, file);
                if let Some(piece) = self.piece_at(square) {
                    if empty_squares != 0 {
                        write!(&mut buf, "{}", empty_squares).unwrap();
                    }
                    write!(&mut buf, "{}", piece).unwrap();
                    empty_squares = 0;
                } else {
                    empty_squares += 1;
                }
            }

            if empty_squares != 0 {
                write!(&mut buf, "{}", empty_squares).unwrap();
            }

            if rank != core::RANK_1 {
                buf.push('/');
            }
        }

        buf.push(' ');
        match self.side_to_move() {
            Color::White => buf.push('w'),
            Color::Black => buf.push('b'),
        }
        buf.push(' ');
        if self.can_castle_kingside(Color::White) {
            buf.push('K');
        }
        if self.can_castle_queenside(Color::White) {
            buf.push('Q');
        }
        if self.can_castle_kingside(Color::Black) {
            buf.push('k');
        }
        if self.can_castle_queenside(Color::Black) {
            buf.push('q');
        }
        buf.push(' ');
        if let Some(ep_square) = self.en_passant_square() {
            write!(&mut buf, "{}", ep_square).unwrap();
        } else {
            buf.push('-');
        }
        buf.push(' ');
        write!(
            &mut buf,
            "{} {}",
            self.halfmove_clock(),
            self.fullmove_clock()
        )
        .unwrap();
        buf
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for rank in core::ranks().rev() {
            for file in core::files() {
                let sq = Square::of(rank, file);
                if let Some(piece) = self.piece_at(sq) {
                    write!(f, " {} ", piece)?;
                } else {
                    write!(f, " . ")?;
                }
            }

            writeln!(f, "| {}", rank)?;
        }

        for _ in core::files() {
            write!(f, "---")?;
        }

        writeln!(f)?;
        for file in core::files() {
            write!(f, " {} ", file)?;
        }

        writeln!(f)?;
        Ok(())
    }
}

impl Default for Position {
    fn default() -> Self {
        Position::new()
    }
}

#[cfg(test)]
mod tests {
    mod fen {
        use std::convert::TryFrom;

        use crate::core::*;
        use crate::position::{FenParseError, Position};

        #[test]
        fn fen_smoke() {
            let pos = Position::from_fen("8/8/8/8/8/8/8/8 w - - 0 0").unwrap();

            // white's turn to move.
            assert_eq!(Color::White, pos.side_to_move());

            // no castling.
            assert!(!pos.can_castle_kingside(Color::White));
            assert!(!pos.can_castle_kingside(Color::Black));
            assert!(!pos.can_castle_queenside(Color::White));
            assert!(!pos.can_castle_queenside(Color::Black));

            // no en passant.
            assert!(pos.en_passant_square().is_none());

            // both clocks are zero.
            assert_eq!(0, pos.halfmove_clock());
            assert_eq!(0, pos.fullmove_clock());
        }

        #[test]
        fn starting_position() {
            let pos =
                Position::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
                    .unwrap();

            let check_square = |square: &'static str, piece: Piece| {
                assert!(square.len() == 2);
                let chars: Vec<_> = square.chars().collect();
                let file = File::try_from(chars[0]).unwrap();
                let rank = Rank::try_from(chars[1]).unwrap();
                let square = Square::of(rank, file);
                let piece_on_square = pos.piece_at(square).unwrap();
                assert_eq!(piece.kind, piece_on_square.kind);
                assert_eq!(piece.color, piece_on_square.color);
            };

            let expected_vacant_squares = SquareSet::all().rank(RANK_3)
                | SquareSet::all().rank(RANK_4)
                | SquareSet::all().rank(RANK_5)
                | SquareSet::all().rank(RANK_6);

            let check_vacant = |square: Square| {
                assert!(pos.piece_at(square).is_none());
            };

            check_square(
                "a1",
                Piece {
                    kind: PieceKind::Rook,
                    color: Color::White,
                },
            );
            check_square(
                "b1",
                Piece {
                    kind: PieceKind::Knight,
                    color: Color::White,
                },
            );
            check_square(
                "c1",
                Piece {
                    kind: PieceKind::Bishop,
                    color: Color::White,
                },
            );
            check_square(
                "d1",
                Piece {
                    kind: PieceKind::Queen,
                    color: Color::White,
                },
            );
            check_square(
                "e1",
                Piece {
                    kind: PieceKind::King,
                    color: Color::White,
                },
            );
            check_square(
                "f1",
                Piece {
                    kind: PieceKind::Bishop,
                    color: Color::White,
                },
            );
            check_square(
                "g1",
                Piece {
                    kind: PieceKind::Knight,
                    color: Color::White,
                },
            );
            check_square(
                "h1",
                Piece {
                    kind: PieceKind::Rook,
                    color: Color::White,
                },
            );
            check_square(
                "a2",
                Piece {
                    kind: PieceKind::Pawn,
                    color: Color::White,
                },
            );
            check_square(
                "b2",
                Piece {
                    kind: PieceKind::Pawn,
                    color: Color::White,
                },
            );
            check_square(
                "c2",
                Piece {
                    kind: PieceKind::Pawn,
                    color: Color::White,
                },
            );
            check_square(
                "d2",
                Piece {
                    kind: PieceKind::Pawn,
                    color: Color::White,
                },
            );
            check_square(
                "e2",
                Piece {
                    kind: PieceKind::Pawn,
                    color: Color::White,
                },
            );
            check_square(
                "f2",
                Piece {
                    kind: PieceKind::Pawn,
                    color: Color::White,
                },
            );
            check_square(
                "g2",
                Piece {
                    kind: PieceKind::Pawn,
                    color: Color::White,
                },
            );
            check_square(
                "h2",
                Piece {
                    kind: PieceKind::Pawn,
                    color: Color::White,
                },
            );

            for sq in expected_vacant_squares {
                let sq_actual = Square::try_from(sq).unwrap();
                check_vacant(sq_actual);
            }

            check_square(
                "a7",
                Piece {
                    kind: PieceKind::Pawn,
                    color: Color::Black,
                },
            );
            check_square(
                "b7",
                Piece {
                    kind: PieceKind::Pawn,
                    color: Color::Black,
                },
            );
            check_square(
                "c7",
                Piece {
                    kind: PieceKind::Pawn,
                    color: Color::Black,
                },
            );
            check_square(
                "d7",
                Piece {
                    kind: PieceKind::Pawn,
                    color: Color::Black,
                },
            );
            check_square(
                "e7",
                Piece {
                    kind: PieceKind::Pawn,
                    color: Color::Black,
                },
            );
            check_square(
                "f7",
                Piece {
                    kind: PieceKind::Pawn,
                    color: Color::Black,
                },
            );
            check_square(
                "g7",
                Piece {
                    kind: PieceKind::Pawn,
                    color: Color::Black,
                },
            );
            check_square(
                "h7",
                Piece {
                    kind: PieceKind::Pawn,
                    color: Color::Black,
                },
            );
            check_square(
                "a8",
                Piece {
                    kind: PieceKind::Rook,
                    color: Color::Black,
                },
            );
            check_square(
                "b8",
                Piece {
                    kind: PieceKind::Knight,
                    color: Color::Black,
                },
            );
            check_square(
                "c8",
                Piece {
                    kind: PieceKind::Bishop,
                    color: Color::Black,
                },
            );
            check_square(
                "d8",
                Piece {
                    kind: PieceKind::Queen,
                    color: Color::Black,
                },
            );
            check_square(
                "e8",
                Piece {
                    kind: PieceKind::King,
                    color: Color::Black,
                },
            );
            check_square(
                "f8",
                Piece {
                    kind: PieceKind::Bishop,
                    color: Color::Black,
                },
            );
            check_square(
                "g8",
                Piece {
                    kind: PieceKind::Knight,
                    color: Color::Black,
                },
            );
            check_square(
                "h8",
                Piece {
                    kind: PieceKind::Rook,
                    color: Color::Black,
                },
            );

            assert!(pos.can_castle_kingside(Color::White));
            assert!(pos.can_castle_kingside(Color::Black));
            assert!(pos.can_castle_queenside(Color::White));
            assert!(pos.can_castle_queenside(Color::Black));
        }

        #[test]
        fn empty() {
            let err = Position::from_fen("").unwrap_err();
            assert_eq!(FenParseError::UnexpectedEnd, err);
        }

        #[test]
        fn unknown_piece() {
            let err = Position::from_fen("z7/8/8/8/8/8/8/8 w - - 0 0").unwrap_err();
            assert_eq!(FenParseError::UnknownPiece('z'), err);
        }

        #[test]
        fn invalid_digit() {
            let err = Position::from_fen("9/8/8/8/8/8/8/8 w - - 0 0").unwrap_err();
            assert_eq!(FenParseError::InvalidDigit, err);
        }

        #[test]
        fn not_sum_to_8() {
            let err = Position::from_fen("pppp5/8/8/8/8/8/8/8 w - - 0 0").unwrap_err();
            assert_eq!(FenParseError::FileDoesNotSumToEight, err);
        }

        #[test]
        fn bad_side_to_move() {
            let err = Position::from_fen("8/8/8/8/8/8/8/8 c - - 0 0").unwrap_err();
            assert_eq!(FenParseError::InvalidSideToMove, err);
        }

        #[test]
        fn bad_castle_status() {
            let err = Position::from_fen("8/8/8/8/8/8/8/8 w a - 0 0").unwrap_err();
            assert_eq!(FenParseError::InvalidCastle, err);
        }

        #[test]
        fn bad_en_passant() {
            let err = Position::from_fen("8/8/8/8/8/8/8/8 w - 88 0 0").unwrap_err();
            assert_eq!(FenParseError::InvalidEnPassant, err);
        }

        #[test]
        fn empty_halfmove() {
            let err = Position::from_fen("8/8/8/8/8/8/8/8 w - - q 0").unwrap_err();
            assert_eq!(FenParseError::EmptyHalfmove, err);
        }

        #[test]
        fn invalid_halfmove() {
            let err = Position::from_fen("8/8/8/8/8/8/8/8 w - - 4294967296 0").unwrap_err();
            assert_eq!(FenParseError::InvalidHalfmove, err);
        }

        #[test]
        fn empty_fullmove() {
            let err = Position::from_fen("8/8/8/8/8/8/8/8 w - - 0 q").unwrap_err();
            assert_eq!(FenParseError::EmptyFullmove, err);
        }

        #[test]
        fn fullmove_early_end() {
            let err = Position::from_fen("8/8/8/8/8/8/8/8 w - - 0").unwrap_err();
            assert_eq!(FenParseError::UnexpectedEnd, err);
        }

        #[test]
        fn invalid_fullmove() {
            let err = Position::from_fen("8/8/8/8/8/8/8/8 w - - 0 4294967296").unwrap_err();
            assert_eq!(FenParseError::InvalidFullmove, err);
        }

        #[test]
        fn start_position_roundtrip() {
            let str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
            let pos = Position::from_fen(str).unwrap();
            assert_eq!(pos.as_fen(), str);
        }

        /*
        #[test]
        fn uci_nullmove() {
            let pos = Position::from_start_position();
            assert_eq!(Move::null(), pos.move_from_uci("0000").unwrap());
        }

        #[test]
        fn uci_sliding_moves() {
            let pos = Position::from_fen("8/3q4/8/8/8/3R4/8/8 w - - 0 1").unwrap();
            assert_eq!(
                Move::quiet(Square::D3, Square::D5),
                pos.move_from_uci("d3d5").unwrap()
            );
            assert_eq!(
                Move::capture(Square::D3, Square::D7),
                pos.move_from_uci("d3d7").unwrap()
            );
        }

        #[test]
        fn uci_pawn_moves() {
            let pos = Position::from_fen("8/8/8/8/8/4p3/3P4/8 w - c3 0 1").unwrap();
            assert_eq!(
                Move::quiet(Square::D2, Square::D3),
                pos.move_from_uci("d2d3").unwrap()
            );
            assert_eq!(
                Move::double_pawn_push(Square::D2, Square::D4),
                pos.move_from_uci("d2d4").unwrap()
            );
            assert_eq!(
                Move::capture(Square::D2, Square::E3),
                pos.move_from_uci("d2e3").unwrap()
            );
            assert_eq!(
                Move::quiet(Square::D2, Square::D3),
                pos.move_from_uci("d2d3").unwrap()
            );
            assert_eq!(
                Move::en_passant(Square::D2, Square::C3),
                pos.move_from_uci("d2c3").unwrap()
            );
        }

        #[test]
        fn uci_king_moves() {
            let pos = Position::from_fen("8/8/8/8/8/8/3r4/R3K2R w - - 0 1").unwrap();
            assert_eq!(
                Move::kingside_castle(Square::E1, Square::G1),
                pos.move_from_uci("e1g1").unwrap(),
            );
            assert_eq!(
                Move::queenside_castle(Square::E1, Square::C1),
                pos.move_from_uci("e1c1").unwrap(),
            );
            assert_eq!(
                Move::quiet(Square::E1, Square::E2),
                pos.move_from_uci("e1e2").unwrap(),
            );
            assert_eq!(
                Move::capture(Square::E1, Square::D2),
                pos.move_from_uci("e1d2").unwrap(),
            );
        }

        #[test]
        fn uci_promotion() {
            let pos = Position::from_fen("5n2/4P3/8/8/8/8/8/8 w - - 0 1").unwrap();
            assert_eq!(
                Move::promotion(Square::E7, Square::E8, PieceKind::Knight),
                pos.move_from_uci("e7e8n").unwrap()
            );
            assert_eq!(
                Move::promotion(Square::E7, Square::E8, PieceKind::Bishop),
                pos.move_from_uci("e7e8b").unwrap()
            );
            assert_eq!(
                Move::promotion(Square::E7, Square::E8, PieceKind::Rook),
                pos.move_from_uci("e7e8r").unwrap()
            );
            assert_eq!(
                Move::promotion(Square::E7, Square::E8, PieceKind::Queen),
                pos.move_from_uci("e7e8q").unwrap()
            );
            assert_eq!(
                Move::promotion_capture(Square::E7, Square::F8, PieceKind::Knight),
                pos.move_from_uci("e7f8n").unwrap()
            );
            assert_eq!(
                Move::promotion_capture(Square::E7, Square::F8, PieceKind::Bishop),
                pos.move_from_uci("e7f8b").unwrap()
            );
            assert_eq!(
                Move::promotion_capture(Square::E7, Square::F8, PieceKind::Rook),
                pos.move_from_uci("e7f8r").unwrap()
            );
            assert_eq!(
                Move::promotion_capture(Square::E7, Square::F8, PieceKind::Queen),
                pos.move_from_uci("e7f8q").unwrap()
            );
        }
        */
    }

    mod make_unmake {
        use crate::core::*;
        use crate::position::Position;

        // White pawn on e3, white to move, white moves to e4. It should now be black's turn and there should be
        // a white pawn on e4. Unmake the move, it should be white's turn to move and a white pawn should be on e3.
        #[test]
        fn basic_movement() {
            let mut pos = Position::from_fen("8/8/8/8/8/4P3/8/8 w - - 0 1").unwrap();
            assert!(pos.piece_at(E3).is_some());
            let mov = Move::quiet(E3, E4);
            pos.make_move(mov);
            assert!(pos.piece_at(E3).is_none());
            assert!(pos.piece_at(E4).is_some());
            let piece = pos.piece_at(E4).unwrap();
            assert_eq!(piece.kind, PieceKind::Pawn);
            assert_eq!(piece.color, Color::White);
            assert_eq!(pos.side_to_move(), Color::Black);
            pos.unmake_move(mov);
            assert!(pos.side_to_move() == Color::White);
            assert!(pos.piece_at(E4).is_none());
            assert!(pos.piece_at(E3).is_some());
            let unmake_piece = pos.piece_at(E3).unwrap();
            assert_eq!(unmake_piece.kind, PieceKind::Pawn);
            assert_eq!(unmake_piece.color, Color::White);
        }
    }
}
