// Copyright 2021 Sean Gillespie.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::core::{self, Direction, File, Rank, Square};
use std::fmt;
use std::ops;

/// A set of squares on the chessboard. The implementation of SquareSet is designed to mirror
/// [`std::collections::HashSet`], but is specifically designed to store squares efficiently on modern processors.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct SquareSet(u64);

impl SquareSet {
    /// Creates a new, empty SquareSet.
    pub const fn empty() -> SquareSet {
        SquareSet(0)
    }

    /// Creates a new SquareSet with all squares present in the set.
    pub const fn all() -> SquareSet {
        SquareSet(0xFFFFFFFFFFFFFFFF)
    }

    /// Tests whether or not the given square is contained within this SquareSet.
    pub const fn contains(&self, square: Square) -> bool {
        self.0 & (1u64 << square.0) != 0
    }

    pub fn insert(&mut self, square: Square) {
        self.0 |= 1u64 << square.0;
    }

    pub fn remove(&mut self, square: Square) {
        self.0 &= !(1u64 << square.0);
    }

    pub const fn len(&self) -> u32 {
        self.0.count_ones()
    }

    pub const fn is_empty(&self) -> bool {
        self.0 == 0
    }

    pub const fn and(self, other: SquareSet) -> SquareSet {
        SquareSet(self.0 & other.0)
    }

    pub const fn or(self, other: SquareSet) -> SquareSet {
        SquareSet(self.0 | other.0)
    }

    pub const fn not(self) -> SquareSet {
        SquareSet(!self.0)
    }

    pub const fn xor(self, other: SquareSet) -> SquareSet {
        SquareSet(self.0 ^ other.0)
    }

    pub const fn rank(self, rank: Rank) -> SquareSet {
        let rank_set = match rank {
            core::RANK_1 => SS_RANK_1,
            core::RANK_2 => SS_RANK_2,
            core::RANK_3 => SS_RANK_3,
            core::RANK_4 => SS_RANK_4,
            core::RANK_5 => SS_RANK_5,
            core::RANK_6 => SS_RANK_6,
            core::RANK_7 => SS_RANK_7,
            core::RANK_8 => SS_RANK_8,
            _ => unreachable!(),
        };

        self.and(rank_set)
    }

    pub const fn file(self, file: File) -> SquareSet {
        let file_set = match file {
            core::FILE_A => SS_FILE_A,
            core::FILE_B => SS_FILE_B,
            core::FILE_C => SS_FILE_C,
            core::FILE_D => SS_FILE_D,
            core::FILE_E => SS_FILE_E,
            core::FILE_F => SS_FILE_F,
            core::FILE_G => SS_FILE_G,
            core::FILE_H => SS_FILE_H,
            _ => unreachable!(),
        };

        self.and(file_set)
    }

    /// Shifts all squares in the SquareSet one square in the given direction.
    pub const fn shift(self, direction: Direction) -> SquareSet {
        match direction {
            Direction::North => SquareSet(self.0 << 8),
            Direction::NorthEast => SquareSet(self.and(SS_FILE_H.not()).0 << 9),
            Direction::East => SquareSet(self.and(SS_FILE_H.not()).0 << 1),
            Direction::SouthEast => SquareSet(self.and(SS_FILE_H.not()).0 >> 7),
            Direction::South => SquareSet(self.0 >> 8),
            Direction::SouthWest => SquareSet(self.and(SS_FILE_A.not()).0 >> 9),
            Direction::West => SquareSet(self.and(SS_FILE_A.not()).0 >> 1),
            Direction::NorthWest => SquareSet(self.and(SS_FILE_A.not()).0 << 7),
        }
    }

    pub fn bits(self) -> u64 {
        self.0
    }
}

impl ops::BitOr for SquareSet {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        self.or(rhs)
    }
}

impl ops::Not for SquareSet {
    type Output = Self;

    fn not(self) -> Self::Output {
        self.not()
    }
}

impl ops::BitAnd for SquareSet {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        self.and(rhs)
    }
}

impl ops::BitXor for SquareSet {
    type Output = Self;

    fn bitxor(self, rhs: Self) -> Self::Output {
        self.xor(rhs)
    }
}

impl IntoIterator for SquareSet {
    type Item = Square;
    type IntoIter = SquareSetIterator;

    fn into_iter(self) -> Self::IntoIter {
        SquareSetIterator(self.0)
    }
}

impl fmt::Display for SquareSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for rank in core::ranks().rev() {
            for file in core::files() {
                let sq = Square::of(rank, file);
                if self.contains(sq) {
                    write!(f, " 1 ")?;
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

pub const SS_RANK_1: SquareSet = SquareSet(0x00000000000000FF);
pub const SS_RANK_2: SquareSet = SquareSet(0x000000000000FF00);
pub const SS_RANK_3: SquareSet = SquareSet(0x0000000000FF0000);
pub const SS_RANK_4: SquareSet = SquareSet(0x00000000FF000000);
pub const SS_RANK_5: SquareSet = SquareSet(0x000000FF00000000);
pub const SS_RANK_6: SquareSet = SquareSet(0x0000FF0000000000);
pub const SS_RANK_7: SquareSet = SquareSet(0x00FF000000000000);
pub const SS_RANK_8: SquareSet = SquareSet(0xFF00000000000000);
pub const SS_FILE_A: SquareSet = SquareSet(0x0101010101010101);
pub const SS_FILE_B: SquareSet = SquareSet(0x0202020202020202);
pub const SS_FILE_C: SquareSet = SquareSet(0x0404040404040404);
pub const SS_FILE_D: SquareSet = SquareSet(0x0808080808080808);
pub const SS_FILE_E: SquareSet = SquareSet(0x1010101010101010);
pub const SS_FILE_F: SquareSet = SquareSet(0x2020202020202020);
pub const SS_FILE_G: SquareSet = SquareSet(0x4040404040404040);
pub const SS_FILE_H: SquareSet = SquareSet(0x8080808080808080);

/// An iterator over squares stored in a [`SquareSet`], designed to be very efficient for modern processors.
pub struct SquareSetIterator(u64);

impl Iterator for SquareSetIterator {
    type Item = Square;

    fn next(&mut self) -> Option<Self::Item> {
        if self.0 == 0 {
            None
        } else {
            let next = self.0.trailing_zeros() as u8;
            self.0 &= self.0 - 1;
            Some(Square(next))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SquareSet;
    use crate::core::*;

    #[test]
    fn test_set_clear() {
        let mut set = SquareSet::empty();
        assert!(!set.contains(A1));
        set.insert(A1);
        assert!(set.contains(A1));
        set.remove(A1);
        assert!(!set.contains(A1));
    }

    #[test]
    fn count() {
        let mut set = SquareSet::empty();
        set.insert(A3);
        set.insert(A4);
        set.insert(A5);
        assert_eq!(set.len(), 3);
    }

    #[test]
    fn iter() {
        let mut set = SquareSet::empty();
        set.insert(A3);
        set.insert(A4);
        set.insert(A5);
        let squares: Vec<_> = set.into_iter().collect();
        assert_eq!(squares, vec![A3, A4, A5]);
    }

    #[test]
    fn rank() {
        let set = SquareSet::all();
        assert!(!set.rank(RANK_7).is_empty());
    }

    #[test]
    fn shift_up() {
        let rank_1 = SquareSet::all().rank(RANK_1);
        let rank_2 = rank_1.shift(Direction::North);
        assert_eq!(rank_2, SquareSet::all().rank(RANK_2))
    }

    #[test]
    fn shift_left() {
        let file_c = SquareSet::all().file(FILE_C);
        let file_b = file_c.shift(Direction::West);
        assert_eq!(file_b, SquareSet::all().file(FILE_B));
    }

    #[test]
    fn shift_upright() {
        let mut set = SquareSet::empty();
        set.insert(H6);
        let result = set.shift(Direction::NorthEast);
        assert!(result.is_empty());
    }
}
