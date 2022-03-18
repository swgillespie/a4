// Copyright 2017-2021 Sean Gillespie.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
use crate::core::SquareSet;
use crate::core::*;
use crate::core::{
    SS_FILES, SS_FILE_A, SS_FILE_B, SS_FILE_C, SS_FILE_D, SS_FILE_E, SS_FILE_F, SS_FILE_G,
    SS_FILE_H, SS_RANKS,
};
use crate::movegen;
use crate::position::Position;

/// Provider of common board analyses upon a static position. It is suitable for use in board
/// evaluators, where analysis queries can be aggressively cached when evaluating a single,
/// immutable board position.
pub struct Analysis<'a> {
    pos: &'a Position,
}

impl<'a> Analysis<'a> {
    pub fn new(pos: &'a Position) -> Analysis<'a> {
        Analysis { pos }
    }

    /// Returns the set of doubled pawns left by the given color.
    pub fn doubled_pawns(&self, color: Color) -> SquareSet {
        let pawns = self.pos.pawns(color);
        let mut answer = SquareSet::empty();
        for &file in &SS_FILES {
            let pawns_on_file = pawns.and(file);
            if pawns_on_file.len() > 1 {
                answer = answer.or(pawns_on_file);
            }
        }

        answer
    }

    /// Returns the set of backward pawns left by the given color.
    pub fn backward_pawns(&self, color: Color) -> SquareSet {
        fn walk_rank<I>(
            iter: I,
            current_file_pawns: SquareSet,
            adjacent_file_pawns: SquareSet,
        ) -> SquareSet
        where
            I: Iterator<Item = SquareSet>,
        {
            let mut answer = SquareSet::empty();
            for rank in iter {
                let current_file_rank = rank.and(current_file_pawns);
                let adjacent_file_rank = rank.and(adjacent_file_pawns);
                if !current_file_rank.is_empty() && adjacent_file_rank.is_empty() {
                    answer = answer.or(current_file_rank);
                    break;
                }

                if !adjacent_file_rank.is_empty() && current_file_rank.is_empty() {
                    break;
                }
            }

            answer
        }

        let pawns = self.pos.pawns(color);
        let mut answer = SquareSet::empty();
        for file in files() {
            let adj_files = adjacent_files(file);
            let current_file = SquareSet::all().file(file);
            let pawns_on_current_file = pawns.and(current_file);
            let pawns_on_adjacent_files = pawns.and(adj_files);
            if pawns_on_current_file.is_empty() {
                continue;
            }

            let file_answer = match color {
                Color::White => walk_rank(
                    SS_RANKS.iter().cloned(),
                    pawns_on_current_file,
                    pawns_on_adjacent_files,
                ),
                Color::Black => walk_rank(
                    SS_RANKS.iter().cloned().rev(),
                    pawns_on_current_file,
                    pawns_on_adjacent_files,
                ),
            };

            answer = answer.or(file_answer);
        }

        answer
    }

    pub fn isolated_pawns(&self, color: Color) -> SquareSet {
        let pawns = self.pos.pawns(color);
        let mut answer = SquareSet::empty();
        for file in files() {
            let adj_files = adjacent_files(file);
            let current_file = SquareSet::all().file(file);
            let pawns_on_current_file = pawns.and(current_file);
            let pawns_on_adjacent_file = pawns.and(adj_files);
            if pawns_on_current_file.is_empty() {
                continue;
            }

            if pawns_on_adjacent_file.is_empty() {
                answer = answer.or(pawns_on_current_file);
            }
        }

        answer
    }

    pub fn mobility(&self, color: Color) -> u32 {
        // Our move generator only operates on the current side to move. If we need to analyze the
        // other side, make a null move and analyze that instead.
        let pos = if self.pos.side_to_move() != color {
            let mut copied_pos = self.pos.clone();
            copied_pos.make_move(Move::null());
            copied_pos
        } else {
            self.pos.clone()
        };

        assert!(pos.side_to_move() == color);
        let mut moves = Vec::new();
        movegen::generate_moves(pos.side_to_move(), &pos, &mut moves);
        let mut count = 0;
        for &mov in &moves {
            if pos.is_legal_given_pseudolegal(mov) {
                count += 1;
            }
        }

        count
    }
}

fn adjacent_files(file: File) -> SquareSet {
    match file {
        FILE_A => SS_FILE_B,
        FILE_B => SS_FILE_A.or(SS_FILE_C),
        FILE_C => SS_FILE_B.or(SS_FILE_D),
        FILE_D => SS_FILE_C.or(SS_FILE_E),
        FILE_E => SS_FILE_D.or(SS_FILE_F),
        FILE_F => SS_FILE_E.or(SS_FILE_G),
        FILE_G => SS_FILE_F.or(SS_FILE_H),
        FILE_H => SS_FILE_G,
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use super::Analysis;

    use crate::core::*;
    use crate::position::Position;

    #[test]
    fn doubled_pawn_smoke() {
        let pos = Position::from_fen("8/6P1/2P5/4P3/2P2P2/PP1P2P1/P7/8 w - - 0 1").unwrap();
        let analysis = Analysis::new(&pos);
        let doubled_pawns = analysis.doubled_pawns(Color::White);

        assert!(doubled_pawns.contains(A2));
        assert!(doubled_pawns.contains(A3));

        assert!(!doubled_pawns.contains(B3));

        assert!(doubled_pawns.contains(C4));
        assert!(doubled_pawns.contains(C6));

        assert!(!doubled_pawns.contains(D3));
        assert!(!doubled_pawns.contains(E5));
        assert!(!doubled_pawns.contains(F4));

        assert!(doubled_pawns.contains(G3));
        assert!(doubled_pawns.contains(G7));
    }

    #[test]
    fn backward_pawn_smoke() {
        let pos = Position::from_fen("8/8/8/8/8/2P1P3/3P4/8 w - - 0 1").unwrap();
        let analysis = Analysis::new(&pos);
        let backward_pawns = analysis.backward_pawns(Color::White);
        assert_eq!(1, backward_pawns.len());
        assert!(backward_pawns.contains(D2));
    }

    #[test]
    fn backward_pawn_smoke_black() {
        let pos = Position::from_fen("8/3p4/2p1p3/8/8/8/8/8 b - - 0 1").unwrap();
        let analysis = Analysis::new(&pos);
        let backward_pawns = analysis.backward_pawns(Color::Black);
        assert_eq!(1, backward_pawns.len());
        assert!(backward_pawns.contains(D7));
    }

    #[test]
    fn mobility_smoke() {
        let pos = Position::from_fen("8/8/4r3/8/8/4B3/4K3/8 w - - 0 1").unwrap();
        let analysis = Analysis::new(&pos);

        // White's bishop is not allowed to move at all, since it is absolutely pinned by the Black
        // rook. As a result, its mobility score is low, despite having more pieces on the board.
        assert_eq!(7, analysis.mobility(Color::White));
        assert_eq!(12, analysis.mobility(Color::Black));
    }

    #[test]
    fn isolated_pawn_smoke() {
        let pos = Position::from_fen("8/8/8/8/8/3P1P2/6P1/8 w - - 0 1").unwrap();
        let analysis = Analysis::new(&pos);
        let isolated_pawns = analysis.isolated_pawns(Color::White);
        assert_eq!(1, isolated_pawns.len());
        assert!(isolated_pawns.contains(D3));
    }
}
