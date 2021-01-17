// Copyright 2021 Sean Gillespie.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::{convert::TryFrom, fmt};
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
}

#[derive(Debug, Error)]
pub enum FileParseError {
    #[error("file index out of range: {0}")]
    OutOfRange(u8),
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

pub struct SquaresIterator(u8);

impl Iterator for SquaresIterator {
    type Item = Square;

    fn next(&mut self) -> Option<Self::Item> {
        if self.0 >= 64 {
            None
        } else {
            let idx = self.0;
            self.0 += 1;
            Some(Square(idx))
        }
    }
}

pub fn squares() -> SquaresIterator {
    SquaresIterator(0)
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
        if value >= 7 {
            return Err(RankParseError::OutOfRange(value));
        }

        Ok(Rank(value))
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

impl TryFrom<u8> for File {
    type Error = FileParseError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value >= 7 {
            return Err(FileParseError::OutOfRange(value));
        }

        Ok(File(value))
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

pub struct Color(u8);

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
