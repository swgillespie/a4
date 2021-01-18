// Copyright 2021 Sean Gillespie.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::core::{self, File, Rank, Square};
use std::ops::BitOr;

/// A set of squares on the chessboard. The implementation of SquareSet is designed to mirror
/// [`std::collections::HashSet`], but is specifically designed to store squares efficiently on modern processors.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct SquareSet(u64);

impl SquareSet {
    /// Creates a new, empty SquareSet.
    pub const fn empty() -> SquareSet {
        SquareSet::bits(0)
    }

    const fn bits(bits: u64) -> SquareSet {
        SquareSet(bits)
    }

    /// Creates a new SquareSet with all squares present in the set.
    pub const fn all() -> SquareSet {
        SquareSet::bits(0xFFFFFFFFFFFF)
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

    pub fn rank(self, rank: Rank) -> SquareSet {
        let rank_set = match rank {
            core::RANK_1 => SquareSet::bits(0x00000000000000FF),
            core::RANK_2 => SquareSet::bits(0x000000000000FF00),
            core::RANK_3 => SquareSet::bits(0x0000000000FF0000),
            core::RANK_4 => SquareSet::bits(0x00000000FF000000),
            core::RANK_5 => SquareSet::bits(0x000000FF00000000),
            core::RANK_6 => SquareSet::bits(0x0000FF0000000000),
            core::RANK_7 => SquareSet::bits(0x00FF000000000000),
            core::RANK_8 => SquareSet::bits(0xFF00000000000000),
            _ => unreachable!(),
        };

        self.and(rank_set)
    }

    pub fn file(self, file: File) -> SquareSet {
        let file_set = match file {
            core::FILE_A => SquareSet::bits(0x00000000000000FF),
            core::FILE_B => SquareSet::bits(0x0202020202020202),
            core::FILE_C => SquareSet::bits(0x0404040404040404),
            core::FILE_D => SquareSet::bits(0x0808080808080808),
            core::FILE_E => SquareSet::bits(0x1010101010101010),
            core::FILE_F => SquareSet::bits(0x2020202020202020),
            core::FILE_G => SquareSet::bits(0x4040404040404040),
            core::FILE_H => SquareSet::bits(0x8080808080808080),
            _ => unreachable!(),
        };

        self.and(file_set)
    }
}

impl BitOr for SquareSet {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        self.or(rhs)
    }
}

impl IntoIterator for SquareSet {
    type Item = Square;
    type IntoIter = SquareSetIterator;

    fn into_iter(self) -> Self::IntoIter {
        SquareSetIterator(self.0)
    }
}

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
    use crate::core;

    #[test]
    fn test_set_clear() {
        let mut set = SquareSet::empty();
        assert!(!set.contains(core::A1));
        set.insert(core::A1);
        assert!(set.contains(core::A1));
        set.remove(core::A1);
        assert!(!set.contains(core::A1));
    }

    #[test]
    fn count() {
        let mut set = SquareSet::empty();
        set.insert(core::A3);
        set.insert(core::A4);
        set.insert(core::A5);
        assert_eq!(set.len(), 3);
    }

    #[test]
    fn iter() {
        let mut set = SquareSet::empty();
        set.insert(core::A3);
        set.insert(core::A4);
        set.insert(core::A5);
        let squares: Vec<_> = set.into_iter().collect();
        assert_eq!(squares, vec![core::A3, core::A4, core::A5]);
    }
}
