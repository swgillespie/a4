// Copyright 2017-2021 Sean Gillespie.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::core::{self, CastleStatus, Color, File, Piece, Rank, Square, SquareSet};
use std::convert::TryFrom;
use thiserror::Error;

/// Information that can't be recovered normally when unmaking a move. When making or unmaking a move, this information
/// is cloned instead of incrementally updated.
///
/// Because this structure is cloned, care must be taken to make this structure as small as possible.
#[derive(Clone, Debug)]
struct IrreversibleInformation {
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
#[derive(Clone)]
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
    pub fn new() -> Position {
        Position {
            sets_by_piece: [SquareSet::empty(); 12],
            sets_by_color: [SquareSet::empty(); 2],
            current_information: IrreversibleInformation {
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
#[derive(Debug, Error)]
pub enum FenParseError {
    #[error("unexpected char: {0}")]
    UnexpectedChar(char),
    #[error("unexpected EOF while reading")]
    UnexpectedEnd,
    #[error("invalid digit")]
    InvalidDigit,
    #[error("file does not sum to 8")]
    FileDoesNotSumToEight,
    #[error("unknown piece")]
    UnknownPiece,
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
            while file <= 8 {
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
                    return Err(FenParseError::UnknownPiece);
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

    /*
    pub fn as_fen(&self) -> String {
        let mut buf = String::new();
        for &rank in RANKS.iter().rev() {
            let mut empty_squares = 0;
            for &file in &FILES {
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

            if rank != Rank::One {
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
    */
}
