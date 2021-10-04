// Copyright 2017-2021 Sean Gillespie.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::{convert::TryFrom, fmt};

use bitflags::bitflags;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SquareParseError {
    #[error("square index out of range: {0}")]
    OutOfRange(u8),
}

#[derive(Debug, Error)]
pub enum RankParseError {
    #[error("rank index out of range: {0}")]
    OutOfRange(u8),
    #[error("invalid char: {0}")]
    InvalidChar(char),
}

#[derive(Debug, Error)]
pub enum FileParseError {
    #[error("file index out of range: {0}")]
    OutOfRange(u8),
    #[error("invalid char: {0}")]
    InvalidChar(char),
}

#[derive(Debug, Error)]
pub enum PieceParseError {
    #[error("invalid char: {0}")]
    InvalidChar(char),
}

/// A square on the chessboard.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Square(pub(in crate::core) u8);

impl Square {
    /// Returns the rank of this square on the chessboard.
    pub const fn rank(self) -> Rank {
        Rank(self.0 >> 3)
    }

    /// Returns the file of this square on the chessboard.
    pub const fn file(self) -> File {
        File(self.0 & 7)
    }

    /// Creates a new Square composed of a given rank and file.
    pub const fn of(rank: Rank, file: File) -> Square {
        Square(rank.0 * 8 + file.0)
    }

    pub(in crate::core) const fn plus(self, offset: i32) -> Square {
        Square((self.0 as i32 + offset) as u8)
    }

    /// Returns the closest square in the given direction. Invalid if the requested direction goes off of the
    /// board.
    pub const fn towards(self, dir: Direction) -> Square {
        self.plus(dir.as_vector())
    }

    pub const fn as_u8(self) -> u8 {
        self.0
    }
}

impl TryFrom<u8> for Square {
    type Error = SquareParseError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value >= 64 {
            return Err(SquareParseError::OutOfRange(value));
        }

        Ok(Square(value))
    }
}

impl fmt::Display for Square {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.file(), self.rank())
    }
}

pub const A1: Square = Square(0);
pub const B1: Square = Square(1);
pub const C1: Square = Square(2);
pub const D1: Square = Square(3);
pub const E1: Square = Square(4);
pub const F1: Square = Square(5);
pub const G1: Square = Square(6);
pub const H1: Square = Square(7);
pub const A2: Square = Square(8);
pub const B2: Square = Square(9);
pub const C2: Square = Square(10);
pub const D2: Square = Square(11);
pub const E2: Square = Square(12);
pub const F2: Square = Square(13);
pub const G2: Square = Square(14);
pub const H2: Square = Square(15);
pub const A3: Square = Square(16);
pub const B3: Square = Square(17);
pub const C3: Square = Square(18);
pub const D3: Square = Square(19);
pub const E3: Square = Square(20);
pub const F3: Square = Square(21);
pub const G3: Square = Square(22);
pub const H3: Square = Square(23);
pub const A4: Square = Square(24);
pub const B4: Square = Square(25);
pub const C4: Square = Square(26);
pub const D4: Square = Square(27);
pub const E4: Square = Square(28);
pub const F4: Square = Square(29);
pub const G4: Square = Square(30);
pub const H4: Square = Square(31);
pub const A5: Square = Square(32);
pub const B5: Square = Square(33);
pub const C5: Square = Square(34);
pub const D5: Square = Square(35);
pub const E5: Square = Square(36);
pub const F5: Square = Square(37);
pub const G5: Square = Square(38);
pub const H5: Square = Square(39);
pub const A6: Square = Square(40);
pub const B6: Square = Square(41);
pub const C6: Square = Square(42);
pub const D6: Square = Square(43);
pub const E6: Square = Square(44);
pub const F6: Square = Square(45);
pub const G6: Square = Square(46);
pub const H6: Square = Square(47);
pub const A7: Square = Square(48);
pub const B7: Square = Square(49);
pub const C7: Square = Square(50);
pub const D7: Square = Square(51);
pub const E7: Square = Square(52);
pub const F7: Square = Square(53);
pub const G7: Square = Square(54);
pub const H7: Square = Square(55);
pub const A8: Square = Square(56);
pub const B8: Square = Square(57);
pub const C8: Square = Square(58);
pub const D8: Square = Square(59);
pub const E8: Square = Square(60);
pub const F8: Square = Square(61);
pub const G8: Square = Square(62);
pub const H8: Square = Square(63);

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Rank(u8);

impl TryFrom<u8> for Rank {
    type Error = RankParseError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value >= 8 {
            return Err(RankParseError::OutOfRange(value));
        }

        Ok(Rank(value))
    }
}

impl TryFrom<char> for Rank {
    type Error = RankParseError;

    fn try_from(value: char) -> Result<Self, Self::Error> {
        let rank = match value {
            '1' => RANK_1,
            '2' => RANK_2,
            '3' => RANK_3,
            '4' => RANK_4,
            '5' => RANK_5,
            '6' => RANK_6,
            '7' => RANK_7,
            '8' => RANK_8,
            c => return Err(RankParseError::InvalidChar(c)),
        };

        Ok(rank)
    }
}

impl fmt::Display for Rank {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let c = match self.0 {
            0 => '1',
            1 => '2',
            2 => '3',
            3 => '4',
            4 => '5',
            5 => '6',
            6 => '7',
            7 => '8',
            _ => unreachable!(),
        };

        write!(f, "{}", c)
    }
}

pub const RANK_1: Rank = Rank(0);
pub const RANK_2: Rank = Rank(1);
pub const RANK_3: Rank = Rank(2);
pub const RANK_4: Rank = Rank(3);
pub const RANK_5: Rank = Rank(4);
pub const RANK_6: Rank = Rank(5);
pub const RANK_7: Rank = Rank(6);
pub const RANK_8: Rank = Rank(7);

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct File(u8);

impl File {
    pub const fn as_u8(self) -> u8 {
        self.0
    }
}

impl TryFrom<u8> for File {
    type Error = FileParseError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value >= 8 {
            return Err(FileParseError::OutOfRange(value));
        }

        Ok(File(value))
    }
}

impl TryFrom<char> for File {
    type Error = FileParseError;

    fn try_from(value: char) -> Result<Self, Self::Error> {
        let file = match value {
            'a' => FILE_A,
            'b' => FILE_B,
            'c' => FILE_C,
            'd' => FILE_D,
            'e' => FILE_E,
            'f' => FILE_F,
            'g' => FILE_G,
            'h' => FILE_H,
            c => return Err(FileParseError::InvalidChar(c)),
        };

        Ok(file)
    }
}

impl fmt::Display for File {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let c = match self.0 {
            0 => 'a',
            1 => 'b',
            2 => 'c',
            3 => 'd',
            4 => 'e',
            5 => 'f',
            6 => 'g',
            7 => 'h',
            _ => unreachable!(),
        };

        write!(f, "{}", c)
    }
}

pub const FILE_A: File = File(0);
pub const FILE_B: File = File(1);
pub const FILE_C: File = File(2);
pub const FILE_D: File = File(3);
pub const FILE_E: File = File(4);
pub const FILE_F: File = File(5);
pub const FILE_G: File = File(6);
pub const FILE_H: File = File(7);

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Color {
    White,
    Black,
}

impl Color {
    pub fn toggle(self) -> Color {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PieceKind {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

impl fmt::Display for PieceKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let c = match self {
            PieceKind::Pawn => 'p',
            PieceKind::Knight => 'n',
            PieceKind::Bishop => 'b',
            PieceKind::Rook => 'r',
            PieceKind::Queen => 'q',
            PieceKind::King => 'k',
        };

        write!(f, "{}", c)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Piece {
    pub color: Color,
    pub kind: PieceKind,
}

impl TryFrom<char> for Piece {
    type Error = PieceParseError;

    fn try_from(value: char) -> Result<Self, Self::Error> {
        let piece = match value {
            'p' => Piece {
                color: Color::Black,
                kind: PieceKind::Pawn,
            },
            'n' => Piece {
                color: Color::Black,
                kind: PieceKind::Knight,
            },
            'b' => Piece {
                color: Color::Black,
                kind: PieceKind::Bishop,
            },
            'r' => Piece {
                color: Color::Black,
                kind: PieceKind::Rook,
            },
            'q' => Piece {
                color: Color::Black,
                kind: PieceKind::Queen,
            },
            'k' => Piece {
                color: Color::Black,
                kind: PieceKind::King,
            },
            'P' => Piece {
                color: Color::White,
                kind: PieceKind::Pawn,
            },
            'N' => Piece {
                color: Color::White,
                kind: PieceKind::Knight,
            },
            'B' => Piece {
                color: Color::White,
                kind: PieceKind::Bishop,
            },
            'R' => Piece {
                color: Color::White,
                kind: PieceKind::Rook,
            },
            'Q' => Piece {
                color: Color::White,
                kind: PieceKind::Queen,
            },
            'K' => Piece {
                color: Color::White,
                kind: PieceKind::King,
            },
            c => return Err(PieceParseError::InvalidChar(c)),
        };

        Ok(piece)
    }
}

impl fmt::Display for Piece {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let c = match self {
            Piece {
                color: Color::White,
                kind: PieceKind::Pawn,
            } => 'P',
            Piece {
                color: Color::White,
                kind: PieceKind::Knight,
            } => 'N',
            Piece {
                color: Color::White,
                kind: PieceKind::Bishop,
            } => 'B',
            Piece {
                color: Color::White,
                kind: PieceKind::Rook,
            } => 'R',
            Piece {
                color: Color::White,
                kind: PieceKind::Queen,
            } => 'Q',
            Piece {
                color: Color::White,
                kind: PieceKind::King,
            } => 'K',
            Piece {
                color: Color::Black,
                kind: PieceKind::Pawn,
            } => 'p',
            Piece {
                color: Color::Black,
                kind: PieceKind::Knight,
            } => 'n',
            Piece {
                color: Color::Black,
                kind: PieceKind::Bishop,
            } => 'b',
            Piece {
                color: Color::Black,
                kind: PieceKind::Rook,
            } => 'r',
            Piece {
                color: Color::Black,
                kind: PieceKind::Queen,
            } => 'q',
            Piece {
                color: Color::Black,
                kind: PieceKind::King,
            } => 'k',
        };

        write!(f, "{}", c)
    }
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Direction {
    North,
    NorthEast,
    East,
    SouthEast,
    South,
    SouthWest,
    West,
    NorthWest,
}

impl Direction {
    pub const fn as_vector(self) -> i32 {
        match self {
            Direction::North => 8,
            Direction::NorthEast => 9,
            Direction::East => 1,
            Direction::SouthEast => -7,
            Direction::South => -8,
            Direction::SouthWest => -9,
            Direction::West => -1,
            Direction::NorthWest => 7,
        }
    }

    pub const fn reverse(self) -> Direction {
        match self {
            Direction::North => Direction::South,
            Direction::NorthEast => Direction::SouthWest,
            Direction::East => Direction::West,
            Direction::SouthEast => Direction::NorthWest,
            Direction::South => Direction::North,
            Direction::SouthWest => Direction::NorthEast,
            Direction::West => Direction::East,
            Direction::NorthWest => Direction::SouthEast,
        }
    }
}

bitflags! {
    pub struct CastleStatus: u8 {
        const NONE = 0;
        const WHITE_KINGSIDE = 0b0000_0001;
        const WHITE_QUEENSIDE =0b0000_0010;
        const WHITE = Self::WHITE_KINGSIDE.bits | Self::WHITE_QUEENSIDE.bits;
        const BLACK_KINGSIDE = 0b0000_0100;
        const BLACK_QUEENSIDE = 0b0000_1000;
        const BLACK = Self::BLACK_KINGSIDE.bits | Self::BLACK_QUEENSIDE.bits;
    }
}

macro_rules! type_iterator {
    ($name:ident, $type:ident, $max:expr) => {
        pub struct $name(u8, u8);

        impl Iterator for $name {
            type Item = $type;

            fn next(&mut self) -> Option<Self::Item> {
                if self.0 >= self.1 {
                    None
                } else {
                    let next = self.0;
                    self.0 += 1;
                    Some($type(next))
                }
            }
        }

        impl ::std::iter::DoubleEndedIterator for $name {
            fn next_back(&mut self) -> Option<Self::Item> {
                if self.1 == 0 {
                    None
                } else {
                    let next = self.1 - 1;
                    self.1 -= 1;
                    Some($type(next))
                }
            }
        }

        impl Default for $name {
            fn default() -> Self {
                $name(0, $max)
            }
        }
    };
}

type_iterator!(AllSquares, Square, 64);
type_iterator!(AllRanks, Rank, 8);
type_iterator!(AllFiles, File, 8);

pub fn squares() -> AllSquares {
    AllSquares::default()
}

pub fn ranks() -> AllRanks {
    AllRanks::default()
}

pub fn files() -> AllFiles {
    AllFiles::default()
}

pub fn piece_kinds() -> ::std::vec::IntoIter<PieceKind> {
    vec![
        PieceKind::Pawn,
        PieceKind::Bishop,
        PieceKind::Knight,
        PieceKind::Rook,
        PieceKind::Queen,
        PieceKind::King,
    ]
    .into_iter()
}

pub fn colors() -> ::std::vec::IntoIter<Color> {
    vec![Color::White, Color::Black].into_iter()
}
