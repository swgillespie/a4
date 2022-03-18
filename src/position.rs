// Copyright 2017-2021 Sean Gillespie.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::{
    convert::TryFrom,
    fmt::{self, Write},
    hash::{Hash, Hasher},
};

use thiserror::Error;

use crate::{
    core::{self, *},
    movegen, zobrist,
};

/// A position, representing a chess game that has progressed up to this point. A Position encodes the complete state
/// of the game such that the entire game up until this point can be recovered and reconstructed efficiently.
#[derive(Clone, Debug)]
pub struct Position {
    /// SquareSets for each piece and color combination (6 pieces, 2 colors = 12 sets).
    sets_by_piece: [SquareSet; 12],
    /// Squaresets for each color.
    sets_by_color: [SquareSet; 2],
    /// The en-passant square, if the previous move was a double pawn push.
    en_passant_square: Option<Square>,
    /// The halfmove clock, or the progress to a draw by the 50-move Rule.
    halfmove_clock: u16,
    /// The fullmove clock, or number of times white has moved this game.
    fullmove_clock: u16,
    /// Castle status for both players.
    castle_status: CastleStatus,
    /// Color whose turn it is to move.
    side_to_move: Color,
    /// The Zobrist hash of this position.
    zobrist_hash: u64,
}

impl Position {
    pub fn en_passant_square(&self) -> Option<Square> {
        self.en_passant_square
    }

    pub fn halfmove_clock(&self) -> u16 {
        self.halfmove_clock
    }

    pub fn fullmove_clock(&self) -> u16 {
        self.fullmove_clock
    }

    pub fn side_to_move(&self) -> Color {
        self.side_to_move
    }

    pub fn zobrist_hash(&self) -> u64 {
        self.zobrist_hash
    }

    pub fn can_castle_kingside(&self, color: Color) -> bool {
        match color {
            Color::White => self.castle_status.contains(CastleStatus::WHITE_KINGSIDE),
            Color::Black => self.castle_status.contains(CastleStatus::BLACK_KINGSIDE),
        }
    }

    pub fn can_castle_queenside(&self, color: Color) -> bool {
        match color {
            Color::White => self.castle_status.contains(CastleStatus::WHITE_QUEENSIDE),
            Color::Black => self.castle_status.contains(CastleStatus::BLACK_QUEENSIDE),
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

    pub fn king(&self, color: Color) -> Option<Square> {
        let kings = self.pieces_of_kind(color, PieceKind::King);
        assert!(kings.len() <= 1);
        // TODO(swgillespie) this is pretty inefficient
        kings.into_iter().next()
    }
}

impl Position {
    pub fn new() -> Position {
        Position {
            sets_by_piece: [SquareSet::empty(); 12],
            sets_by_color: [SquareSet::empty(); 2],
            halfmove_clock: 0,
            fullmove_clock: 0,
            castle_status: CastleStatus::BLACK | CastleStatus::WHITE,
            en_passant_square: None,
            side_to_move: Color::White,
            zobrist_hash: 0,
        }
    }

    pub fn add_piece(&mut self, square: Square, piece: Piece) -> Result<(), ()> {
        if self.piece_at(square).is_some() {
            return Err(());
        }

        self.sets_by_color[piece.color as usize].insert(square);
        let offset = if piece.color == Color::White { 0 } else { 6 };
        self.sets_by_piece[piece.kind as usize + offset].insert(square);
        zobrist::modify_piece(&mut self.zobrist_hash, square, piece);
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
        zobrist::modify_piece(&mut self.zobrist_hash, square, existing_piece);
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

    pub fn squares_attacking(&self, to_move: Color, target: Square) -> SquareSet {
        // TODO(swgillespie) This function and king move generation need to be rewritten for efficiency
        let mut attacks = SquareSet::empty();

        // Pretend that there's a "super-piece" at the target square and see if it hits anything.
        // This covers all pieces except for kings and pawns.
        let occupancy = self.pieces(Color::White) | self.pieces(Color::Black);

        // Queen attacks cover bishops, rooks, and queens, so check that first.
        let sliding_pieces = self.pieces_of_kind(to_move, PieceKind::Queen)
            | self.pieces_of_kind(to_move, PieceKind::Rook)
            | self.pieces_of_kind(to_move, PieceKind::Bishop);
        let sliding_attacks = queen_attacks(target, occupancy).and(sliding_pieces);
        if !sliding_attacks.is_empty() {
            // Hit - there's something that might be attacking via a slide. However, since we're
            // modeling a superpiece, we need to check that the attacking pieces actually can legally
            // attack this square.
            for attacker in sliding_attacks {
                let piece = self
                    .piece_at(attacker)
                    .expect("attack table produced piece not on board?");
                if core::attacks(piece.kind, piece.color, attacker, occupancy).contains(target) {
                    attacks.insert(attacker);
                }
            }
        }

        // Knight attacks are straightforward since knight moves are symmetric.
        let knight_attacks = knight_attacks(target).and(self.knights(to_move));
        if !knight_attacks.is_empty() {
            attacks = attacks | knight_attacks;
        }

        // For pawns, there are only a few places a pawn could be to legally attack this square. In all cases,
        // the capturing pawn has to be on the rank immediately above (or below) the square we're looking at.
        //
        // A correlary to this is that pieces on the bottom (or top) ranks can't be attacked by pawns.
        let cant_be_attacked_by_pawns_rank = if to_move == Color::White {
            RANK_1
        } else {
            RANK_8
        };

        if target.rank() != cant_be_attacked_by_pawns_rank {
            let pawn_attack_rank = if to_move == Color::White {
                target.towards(Direction::South).rank()
            } else {
                target.towards(Direction::North).rank()
            };
            for pawn in self.pawns(to_move) & SquareSet::all().rank(pawn_attack_rank) {
                if pawn_attacks(pawn, to_move).contains(target) {
                    attacks.insert(pawn);
                }
            }
        }

        // There's only one king, so it's cheap to check.
        if let Some(king) = self.king(to_move) {
            if king_attacks(king).contains(target) {
                attacks.insert(king);
            }
        }

        attacks
    }

    pub fn is_check(&self, us: Color) -> bool {
        if let Some(king) = self.king(us) {
            !self.squares_attacking(us.toggle(), king).is_empty()
        } else {
            false
        }
    }

    /// Legality test for moves that are already known to be pseudolegal. This is strictly faster
    /// than `is_legal`, since `is_legal` also needs to check for pseudo-legality. This method is
    /// useful for legality testing moves coming out of the move generator, which is known to
    /// produce only pseudolegal moves.
    pub fn is_legal_given_pseudolegal(&self, mov: Move) -> bool {
        // The below implementation is naive and simple, but correct. There's lots of room for performance wins here.
        let mut new_pos = self.clone();
        let side = self.side_to_move();
        new_pos.make_move(mov);
        !new_pos.is_check(side)
    }

    /// Legality test for any move. It is generally going to be much faster to use is_legal_given_pseudolegal if you
    /// already know that the machine is pseudolegal.
    pub fn is_legal(&self, mov: Move) -> bool {
        let mut moves = vec![];
        movegen::generate_moves(self.side_to_move, self, &mut moves);
        // O(n) scan here; could be O(1) if we collect moves into a set
        if !moves.contains(&mov) {
            return false;
        }

        self.is_legal_given_pseudolegal(mov)
    }
}

//
// Make and unmake move and associated state update functions.
//

impl Position {
    /// Makes a move on the position, updating all internal state to reflect the effects of the move.
    pub fn make_move(&mut self, mov: Move) {
        // Quick out for null moves:
        //  1. EP is not legal next turn.
        //  2. Halfmove clock always increases.
        //  3. Fullmove clock increases if Black makes the null move.
        if mov.is_null() {
            self.en_passant_square = None;
            self.side_to_move = self.side_to_move.toggle();
            zobrist::modify_side_to_move(&mut self.zobrist_hash);
            if self.side_to_move == Color::White {
                self.fullmove_clock += 1;
            }
            return;
        }

        let moving_piece = self
            .piece_at(mov.source())
            .expect("invalid move: no piece at source square");

        // If this move is a capture, we need to remove the captured piece from the board before we
        // proceed.
        if mov.is_capture() {
            // The target square is often the destination square of the move, except in the case of
            // en-passant where the target square lies on an adjacent file.
            let target_square = if !mov.is_en_passant() {
                mov.destination()
            } else {
                // En-passant moves are the only case when the piece being captured does
                // not lie on the same square as the move destination.
                let ep_dir = if self.side_to_move == Color::White {
                    Direction::South
                } else {
                    Direction::North
                };

                let ep_square = if let Some(ep) = self.en_passant_square {
                    ep
                } else {
                    panic!(
                        "invalid move: EP without EP-square ({}) {}",
                        mov,
                        self.as_fen()
                    );
                };
                ep_square.towards(ep_dir)
            };

            // Remove the piece from the board - it has been captured.
            self.remove_piece(target_square)
                .expect("invalid move: no piece at capture target");

            // If this piece is a rook on its starting square, invalidate the castle for the other
            // player.
            if target_square == kingside_rook(self.side_to_move.toggle()) {
                self.castle_status &= !kingside_castle_mask(self.side_to_move.toggle());
                zobrist::modify_kingside_castle(&mut self.zobrist_hash, self.side_to_move.toggle());
            } else if target_square == queenside_rook(self.side_to_move.toggle()) {
                self.castle_status &= !queenside_castle_mask(self.side_to_move.toggle());
                zobrist::modify_queenside_castle(
                    &mut self.zobrist_hash,
                    self.side_to_move.toggle(),
                );
            }
        }

        // The move destination square is now guaranteed to be empty. Next we need to handle moves
        // that end up in places other than the destination square.
        if mov.is_castle() {
            // Castles are encoded using the king's start and stop position. Notably, the rook is
            // not at the move's destination square.
            //
            // Castles are also interesting in that two pieces move, so we'll handle the move of
            // the rook here and handle the movement of the king later on in the function.
            let (post_castle_dir, pre_castle_dir, num_squares) = if mov.is_kingside_castle() {
                (Direction::West, Direction::East, 1)
            } else {
                (Direction::East, Direction::West, 2)
            };

            let new_rook_square = mov.destination().towards(post_castle_dir);
            let mut rook_square = mov.destination();
            for _ in 0..num_squares {
                rook_square = rook_square.towards(pre_castle_dir);
            }

            let rook = self
                .piece_at(rook_square)
                .expect("invalid move: castle without rook");
            self.remove_piece(rook_square).unwrap();
            self.add_piece(new_rook_square, rook)
                .expect("invalid move: piece at rook target square");
        }

        // Now, we're going to add the moving piece to the destination square. Unless this is a
        // promotion, the piece that we add to the destination is the piece that is currently at
        // the source square.
        let piece_to_add = if mov.is_promotion() {
            Piece {
                kind: mov.promotion_piece(),
                color: self.side_to_move,
            }
        } else {
            moving_piece
        };

        self.remove_piece(mov.source())
            .expect("invalid move: no piece at source square");
        self.add_piece(mov.destination(), piece_to_add)
            .expect("invalid move: piece at destination square");
        if mov.is_double_pawn_push() {
            // Double pawn pushes set the en-passant square.
            let ep_dir = if self.side_to_move == Color::White {
                Direction::South
            } else {
                Direction::North
            };

            let ep_square = mov.destination().towards(ep_dir);
            zobrist::modify_en_passant(
                &mut self.zobrist_hash,
                self.en_passant_square,
                Some(ep_square),
            );
            self.en_passant_square = Some(ep_square);
        } else {
            // All other moves clear the en-passant square.
            self.en_passant_square = None;
            zobrist::modify_en_passant(&mut self.zobrist_hash, self.en_passant_square, None);
        }

        // Re-calculate our castle status. Side to move may have invalidated their castle rights
        // by moving their rooks or king.
        if moving_piece.kind == PieceKind::Rook {
            // Moving a rook invalidates the castle on that rook's side of the board.

            if self.can_castle_queenside(self.side_to_move)
                && mov.source() == queenside_rook(self.side_to_move)
            {
                // Move of the queenside rook. Can't castle queenside anymore.
                self.castle_status &= !queenside_castle_mask(self.side_to_move);
                zobrist::modify_queenside_castle(&mut self.zobrist_hash, self.side_to_move);
            } else if self.can_castle_kingside(self.side_to_move)
                && mov.source() == kingside_rook(self.side_to_move)
            {
                // Move of the kingside rook. Can't castle kingside anymore.
                self.castle_status &= !kingside_castle_mask(self.side_to_move);
                zobrist::modify_kingside_castle(&mut self.zobrist_hash, self.side_to_move);
            }
        } else if moving_piece.kind == PieceKind::King {
            // Moving a king invalides the castle on both sides of the board.
            self.castle_status &= !castle_mask(self.side_to_move);
            zobrist::modify_queenside_castle(&mut self.zobrist_hash, self.side_to_move);
            zobrist::modify_kingside_castle(&mut self.zobrist_hash, self.side_to_move);
        }

        self.side_to_move = self.side_to_move.toggle();
        zobrist::modify_side_to_move(&mut self.zobrist_hash);
        if mov.is_capture() || moving_piece.kind == PieceKind::Pawn {
            self.halfmove_clock = 0;
        } else {
            self.halfmove_clock += 1;
        }

        if self.side_to_move == Color::White {
            self.fullmove_clock += 1;
        }
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
        use std::{iter::Peekable, str::Chars};

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
        pos.castle_status = eat_castle_status(iter)?;
        eat(iter, ' ')?;
        pos.en_passant_square = eat_en_passant(iter)?;
        eat(iter, ' ')?;
        pos.halfmove_clock = eat_halfmove(iter)?;
        eat(iter, ' ')?;
        pos.fullmove_clock = eat_fullmove(iter)?;
        Ok(pos)
    }

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

impl Hash for Position {
    fn hash<H>(&self, hasher: &mut H)
    where
        H: Hasher,
    {
        hasher.write_u64(self.zobrist_hash);
    }
}

#[allow(dead_code)]
fn king_start(color: Color) -> Square {
    match color {
        Color::White => E1,
        Color::Black => E8,
    }
}

fn kingside_rook(color: Color) -> Square {
    match color {
        Color::White => H1,
        Color::Black => H8,
    }
}

fn kingside_castle_mask(color: Color) -> CastleStatus {
    match color {
        Color::White => CastleStatus::WHITE_KINGSIDE,
        Color::Black => CastleStatus::BLACK_KINGSIDE,
    }
}

fn queenside_rook(color: Color) -> Square {
    match color {
        Color::White => A1,
        Color::Black => A8,
    }
}

fn queenside_castle_mask(color: Color) -> CastleStatus {
    match color {
        Color::White => CastleStatus::WHITE_QUEENSIDE,
        Color::Black => CastleStatus::BLACK_QUEENSIDE,
    }
}

fn castle_mask(color: Color) -> CastleStatus {
    match color {
        Color::White => CastleStatus::WHITE,
        Color::Black => CastleStatus::BLACK,
    }
}

#[cfg(test)]
mod tests {
    mod fen {
        use std::convert::TryFrom;

        use crate::{
            core::*,
            position::{FenParseError, Position},
        };

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
    }

    mod legality {
        use crate::{core::*, position::Position};

        #[test]
        fn king_pawn_check() {
            let pos = Position::from_fen("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1").unwrap();
            let mov = Move::quiet(A5, B6);
            assert!(!pos.is_legal_given_pseudolegal(mov));
        }

        #[test]
        fn rook_pin() {
            let pos = Position::from_fen("8/8/4r3/8/8/4B3/4K3/8 b - - 0 1").unwrap();
            let mov = Move::capture(E6, E3);
            assert!(pos.is_legal_given_pseudolegal(mov));
        }
    }

    mod make {
        use crate::{core::*, position::Position};

        #[test]
        fn smoke_test_opening_pawn() {
            let mut pos =
                Position::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 2 1")
                    .unwrap();

            // nothing fancy, move a pawn up one.
            pos.make_move(Move::quiet(E2, E3));

            // it should now be Black's turn to move.
            assert_eq!(Color::Black, pos.side_to_move());

            // the fullmove clock shouldn't have incremented
            // (it only increments every Black move)
            assert_eq!(1, pos.fullmove_clock());

            // a pawn moved, so the halfmove clock should be zero.
            assert_eq!(0, pos.halfmove_clock());

            // there should be a pawn on e3
            let pawn = pos.piece_at(E3).unwrap();
            assert_eq!(PieceKind::Pawn, pawn.kind);
            assert_eq!(Color::White, pawn.color);

            // there should not be a pawn on e2
            let not_pawn = pos.piece_at(E2);
            assert!(not_pawn.is_none());
        }

        #[test]
        fn en_passant_reset() {
            // EP square at e3, black to move
            let mut pos = Position::from_fen("8/8/8/8/4Pp2/8/8/8 b - e3 0 1").unwrap();

            // black not taking EP opportunity
            pos.make_move(Move::quiet(F4, F3));

            // EP no longer possible.
            assert_eq!(Color::White, pos.side_to_move());
            assert_eq!(None, pos.en_passant_square());
        }

        #[test]
        fn double_pawn_push_sets_ep() {
            // white to move
            let mut pos = Position::from_fen("8/8/8/8/8/8/4P3/8 w - - 0 1").unwrap();

            // white double-pawn pushes
            pos.make_move(Move::double_pawn_push(E2, E4));

            // now black to move, with EP square set
            assert_eq!(Color::Black, pos.side_to_move());
            assert_eq!(Some(E3), pos.en_passant_square());
        }

        #[test]
        fn basic_capture() {
            let mut pos = Position::from_fen("8/8/8/8/5p2/4P3/8/8 w - - 2 1").unwrap();
            pos.make_move(Move::capture(E3, F4));

            // There should be a white pawn on F4
            let piece = pos.piece_at(F4).unwrap();
            assert_eq!(PieceKind::Pawn, piece.kind);
            assert_eq!(Color::White, piece.color);

            // There should be no piece on E3
            let other_piece = pos.piece_at(E3);
            assert!(other_piece.is_none());

            // The halfmove clock should reset (capture)
            assert_eq!(0, pos.halfmove_clock());
        }

        #[test]
        fn non_pawn_quiet_move() {
            let mut pos = Position::from_fen("8/8/8/8/8/8/4B3/8 w - - 5 2").unwrap();
            pos.make_move(Move::quiet(E2, G4));

            // the halfmove clock should not be reset.
            assert_eq!(6, pos.halfmove_clock());
        }

        #[test]
        fn moving_king_castle_status() {
            let mut pos = Position::from_fen("8/8/8/8/8/8/8/4K2R w KQ - 0 1").unwrap();

            // white's turn to move, white moves its king.
            pos.make_move(Move::quiet(E1, E2));

            // white can't castle anymore.
            assert!(!pos.can_castle_kingside(Color::White));
            assert!(!pos.can_castle_queenside(Color::White));
        }

        #[test]
        fn moving_kingside_rook_castle_status() {
            let mut pos = Position::from_fen("8/8/8/8/8/8/8/4K2R w KQ - 0 1").unwrap();

            // white's turn to move, white moves its kingside rook.
            pos.make_move(Move::quiet(H1, G1));

            // white can't castle kingside anymore
            assert!(!pos.can_castle_kingside(Color::White));
            assert!(pos.can_castle_queenside(Color::White));
        }

        #[test]
        fn moving_queenside_rook_castle_status() {
            let mut pos = Position::from_fen("8/8/8/8/8/8/8/R3K3 w KQ - 0 1").unwrap();

            // white's turn to move, white moves its queenside rook.
            pos.make_move(Move::quiet(A1, B1));

            // white can't castle queenside anymore
            assert!(!pos.can_castle_queenside(Color::White));
            assert!(pos.can_castle_kingside(Color::White));
        }

        #[test]
        fn rook_capture_castle_status() {
            // tests that we can't capture if there's no rook on the target
            // square, even if the rooks themselves never moved (i.e. they
            // were captured on their starting square)
            let mut pos = Position::from_fen("8/8/8/8/8/7r/4P3/R3K2R b KQ - 0 1").unwrap();

            // black to move, black captures the rook at H1
            pos.make_move(Move::capture(H3, H1));

            // white to move, white pushes the pawn
            pos.make_move(Move::double_pawn_push(E2, E4));

            // black to move, black moves the rook
            pos.make_move(Move::quiet(H1, H5));

            // white moves the queenside rook to the kingside rook
            // start location
            pos.make_move(Move::quiet(A1, A2));
            pos.make_move(Move::quiet(H5, H6));
            pos.make_move(Move::quiet(A2, H2));
            pos.make_move(Move::quiet(H6, H7));
            pos.make_move(Move::quiet(H2, H1));

            // white shouldn't be able to castle kingside, despite
            // there being a rook on the kingside rook square
            // and us never moving the kingside rook
            assert!(!pos.can_castle_kingside(Color::White));
        }

        #[test]
        fn en_passant_capture() {
            // tests that we remove an ep-captured piece from its
            // actual location and not try to remove the EP-square
            let mut pos = Position::from_fen("8/8/8/3pP3/8/8/8/8 w - d6 0 1").unwrap();

            // white to move, white EP-captures the pawn
            pos.make_move(Move::en_passant(E5, D6));

            // there should not be a piece at D5 anymore
            let black_pawn = pos.piece_at(D5);
            assert!(black_pawn.is_none());

            // the white pawn should be at the EP-square
            let white_pawn = pos.piece_at(D6).unwrap();
            assert_eq!(Color::White, white_pawn.color);
            assert_eq!(PieceKind::Pawn, white_pawn.kind);
        }

        #[test]
        fn basic_promotion() {
            let mut pos = Position::from_fen("8/4P3/8/8/8/8/8/8 w - - 0 1").unwrap();

            // white to move, white promotes the pawn on e7
            pos.make_move(Move::promotion(E7, E8, PieceKind::Queen));

            // there should be a queen on e8
            let queen = pos.piece_at(E8).unwrap();
            assert_eq!(Color::White, queen.color);
            assert_eq!(PieceKind::Queen, queen.kind);
        }

        #[test]
        fn basic_promote_capture() {
            let mut pos = Position::from_fen("5b2/4P3/8/8/8/8/8/8 w - - 0 1").unwrap();

            // white to move, white promote-captures the pawn on e7 and captures
            // the bishop
            pos.make_move(Move::promotion_capture(E7, F8, PieceKind::Queen));

            // there should be a white queen on f8
            let queen = pos.piece_at(F8).unwrap();
            assert_eq!(Color::White, queen.color);
            assert_eq!(PieceKind::Queen, queen.kind);
        }

        #[test]
        fn queenside_castle() {
            let mut pos = Position::from_fen("8/8/8/8/8/8/8/R3K3 w Q - 0 1").unwrap();

            // white to move, white castles queenside
            pos.make_move(Move::queenside_castle(E1, C1));

            let rook = pos.piece_at(D1).unwrap();
            assert_eq!(Color::White, rook.color);
            assert_eq!(PieceKind::Rook, rook.kind);

            let king = pos.piece_at(C1).unwrap();
            assert_eq!(Color::White, king.color);
            assert_eq!(PieceKind::King, king.kind);
        }

        #[test]
        fn kingside_castle() {
            let mut pos = Position::from_fen("8/8/8/8/8/8/8/4K2R w K - 0 1").unwrap();

            // white to move, white castles kingside
            pos.make_move(Move::kingside_castle(E1, G1));

            let rook = pos.piece_at(F1).unwrap();
            assert_eq!(Color::White, rook.color);
            assert_eq!(PieceKind::Rook, rook.kind);

            let king = pos.piece_at(G1).unwrap();
            assert_eq!(Color::White, king.color);
            assert_eq!(PieceKind::King, king.kind);
        }
    }
}
