// Copyright 2021 Sean Gillespie.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::core::Square;

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
