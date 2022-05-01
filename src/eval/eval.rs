// Copyright 2021 Sean Gillespie.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
use crate::{
    core::*,
    eval::{analysis::Analysis, Value},
    position::Position,
};

const QUEEN_WEIGHT: i16 = 900;
const ROOK_WEIGHT: i16 = 500;
const BISHOP_WEIGHT: i16 = 300;
const KNIGHT_WEIGHT: i16 = 300;
const PAWN_WEIGHT: i16 = 100;
const MOBILITY_WEIGHT: i16 = 4;
const SPACE_WEIGHT: i16 = 13;
const THREATS_WEIGHT: i16 = 7;
const TEMPO_WEIGHT: i16 = 15;

// Pawn piece modifiers
const ISOLATED_PAWN_MODIFIER: i16 = 17;
const BACKWARD_PAWN_MODIFIER: i16 = 10;
const DOUBLED_PAWN_MODIFIER: i16 = 10;

pub struct Evaluator<'a> {
    analysis: Analysis<'a>,
    mobility: [i16; 2],
    material: [i16; 2],
    pawn_modifiers: [i16; 2],
    space: [i16; 2],
    threats: [i16; 2],
    tempo: [i16; 2],
    #[cfg(feature = "trace-eval")]
    remarks: Vec<(Square, &'static str)>,
}

impl<'a> Evaluator<'a> {
    fn new(pos: &'a Position) -> Evaluator<'a> {
        Evaluator {
            analysis: Analysis::new(pos),
            mobility: [0; 2],
            material: [0; 2],
            pawn_modifiers: [0; 2],
            space: [0; 2],
            threats: [0; 2],
            tempo: [0; 2],
            #[cfg(feature = "trace-eval")]
            remarks: vec![],
        }
    }

    fn evaluate(&mut self) -> Value {
        // Check out mobility first - it's possible that a side has been checkmated.
        let white_mobility = self.analysis.mobility(Color::White);
        if white_mobility == 0 {
            if self.analysis.position().is_check(Color::White) {
                return Value::mated_in(0);
            } else {
                return Value::new(0);
            }
        }
        let black_mobility = self.analysis.mobility(Color::Black);
        if black_mobility == 0 {
            if self.analysis.position().is_check(Color::Black) {
                return Value::mate_in(0);
            } else {
                return Value::new(0);
            }
        }

        // Arbitrary term reducing mobility by 4 to try and penalize low-mobility positions.
        self.mobility[Color::White as usize] = (white_mobility - 4) as i16 * MOBILITY_WEIGHT;
        self.mobility[Color::Black as usize] = (black_mobility - 4) as i16 * MOBILITY_WEIGHT;

        for side in colors() {
            for kind in piece_kinds() {
                for square in self.analysis.position().pieces_of_kind(side, kind) {
                    match kind {
                        PieceKind::Pawn => self.evaluate_pawn(side, square),
                        PieceKind::Knight => self.evaluate_knight(side, square),
                        PieceKind::Bishop => self.evaluate_bishop(side, square),
                        PieceKind::Rook => self.evaluate_rook(side, square),
                        PieceKind::Queen => self.evaluate_queen(side, square),
                        PieceKind::King => {}
                    }
                }
            }
        }

        self.tempo[self.analysis.position().side_to_move() as usize] = TEMPO_WEIGHT;
        self.space();
        self.threats();
        let centipawns = self.final_adjustment(
            sum_terms(self.material)
                + sum_terms(self.mobility)
                + sum_terms(self.pawn_modifiers)
                + sum_terms(self.space)
                + sum_terms(self.tempo)
                + sum_terms(self.threats),
        );
        self.dump_evaluation(centipawns);
        Value::new(centipawns)
    }

    fn evaluate_knight(&mut self, side: Color, _square: Square) {
        self.material[side as usize] += KNIGHT_WEIGHT;
    }

    fn evaluate_bishop(&mut self, side: Color, _square: Square) {
        self.material[side as usize] += BISHOP_WEIGHT;
    }

    fn evaluate_rook(&mut self, side: Color, _square: Square) {
        self.material[side as usize] += ROOK_WEIGHT;
    }

    fn evaluate_queen(&mut self, side: Color, _square: Square) {
        self.material[side as usize] += QUEEN_WEIGHT;
    }

    fn evaluate_pawn(&mut self, side: Color, square: Square) {
        self.material[side as usize] += PAWN_WEIGHT;
        if self.analysis.isolated_pawns(side).contains(square) {
            self.pawn_modifiers[side as usize] -= ISOLATED_PAWN_MODIFIER;
            self.remark(square, "pawn is isolated");
        }

        if self.analysis.doubled_pawns(side).contains(square) {
            self.pawn_modifiers[side as usize] -= DOUBLED_PAWN_MODIFIER;
            self.remark(square, "pawn is doubled");
        }

        if self.analysis.backward_pawns(side).contains(square) {
            self.pawn_modifiers[side as usize] -= BACKWARD_PAWN_MODIFIER;
            self.remark(square, "pawn is backward");
        }
    }

    /// Computes the space coefficient for each side. "Space" represents the space that is controlled by a given player
    /// - the space behind their advanced pawns.
    ///
    /// The intention of this term is to encourage the engine to grab space and hold it early in the game to prevent
    /// poor opening play. This term encodes the inutition of "control the center". It does not require that the center
    /// be held with pawns to not discourage hypermodern play.
    fn space(&mut self) {
        for side in colors() {
            let center_files = SS_FILE_C | SS_FILE_D | SS_FILE_E | SS_FILE_F;
            let our_side_of_the_board = match side {
                Color::White => SS_RANK_2 | SS_RANK_3 | SS_RANK_4,
                Color::Black => SS_RANK_7 | SS_RANK_6 | SS_RANK_5,
            };
            let space_squares = center_files & our_side_of_the_board;
            let down = match side {
                Color::White => Direction::South,
                Color::Black => Direction::North,
            };
            let pos = self.analysis.position();

            // Our pawns lead the way into the unknown and claim space; a space is only claimed, though, if it is actually
            // safe and not attacked by our opponent's pawns.
            let safe_squares = space_squares
                & !pos.pawns(side)
                & !self
                    .analysis
                    .attacked_by_kind(side.toggle(), PieceKind::Pawn);
            let mut space_behind_pawns = pos.pawns(side);
            space_behind_pawns = space_behind_pawns | pos.pawns(side).shift(down);
            space_behind_pawns = space_behind_pawns | pos.pawns(side).shift(down).shift(down);
            let totally_safe_spaces =
                safe_squares & space_behind_pawns & !self.analysis.attacked_by(side.toggle());
            self.space[side as usize] =
                (safe_squares.len() as i16 + totally_safe_spaces.len() as i16) * SPACE_WEIGHT;
        }
    }

    /// Threat term for evaluation. The intent of this term is to encode the intuition that it is best to keep your
    /// pieces protected and take penalties whenever our opponent attacks a poorly-defended piece, even if we are able
    /// to deflect the attack in search.
    fn threats(&mut self) {
        let pos = self.analysis.position();
        for side in colors() {
            // Opponent's pieces that are defended are "attacked" by their fellow pieces.
            let defended_pieces =
                pos.pieces(side.toggle()) & self.analysis.attacked_by(side.toggle());

            // Weak pieces are attacked by us and not defended adequately.
            let weak_pieces =
                pos.pieces(side.toggle()) & !defended_pieces & self.analysis.attacked_by(side);
            self.threats[side as usize] = weak_pieces.len() as i16 * THREATS_WEIGHT;
        }
    }

    /// Final adjustment of the centipawn score, based on some late heuristics.
    fn final_adjustment(&mut self, input_cp: i16) -> i16 {
        let winning_side = if input_cp > 0 {
            Color::White
        } else {
            Color::Black
        };

        let pos = self.analysis.position();
        if pos.pawns(winning_side).is_empty() {
            // Winning side has no pawns; the only way they can win is by coordinating
            // minor pieces.
            //
            // A few known rules here:
            // 1. A minor piece alone can't win and can only draw.
            let knights = pos.knights(winning_side);
            let bishops = pos.bishops(winning_side);
            let rooks = pos.rooks(winning_side);
            let queens = pos.queens(winning_side);
            if rooks.is_empty() && queens.is_empty() && (knights.or(bishops).len() == 1) {
                self.remark(A1, "position is draw by insufficient material");
                return 0;
            }

            // 2. Two knights can't checkmate a bare king.
            if pos.pieces(winning_side.toggle()).len() == 1
                && bishops.is_empty()
                && rooks.is_empty()
                && queens.is_empty()
                && knights.len() == 2
            {
                self.remark(A2, "position is draw by insufficient material");
                return 0;
            }

            // 3. Bare king vs bare king is a draw.
            if pos.pieces(winning_side).len() == 1 && pos.pieces(winning_side.toggle()).len() == 1 {
                self.remark(A3, "position is draw by insufficient material");
                return 0;
            }
        }

        return input_cp;
    }

    #[cfg(feature = "trace-eval")]
    fn remark(&mut self, square: Square, remark: &'static str) {
        self.remarks.push((square, remark));
    }

    #[cfg(not(feature = "trace-eval"))]
    fn remark(&mut self, _: Square, _: &'static str) {}

    #[cfg(feature = "trace-eval")]
    fn dump_evaluation(&self, cp: i16) {
        println!("========================================");
        println!("FEN: {}", self.analysis.position().as_fen());
        println!("========================================");
        println!("Term           | White | Black | Total |");
        println!("----------------------------------------");
        println!(
            "Material       | {:^5} | {:^5} | {:^5} |",
            self.material[Color::White as usize],
            self.material[Color::Black as usize],
            sum_terms(self.material)
        );
        println!(
            "Mobility       | {:^5} | {:^5} | {:^5} |",
            self.mobility[Color::White as usize],
            self.mobility[Color::Black as usize],
            sum_terms(self.mobility)
        );
        println!(
            "Pawn Modifiers | {:^5} | {:^5} | {:^5} |",
            self.pawn_modifiers[Color::White as usize],
            self.pawn_modifiers[Color::Black as usize],
            sum_terms(self.pawn_modifiers)
        );
        println!(
            "Space          | {:^5} | {:^5} | {:^5} |",
            self.space[Color::White as usize],
            self.space[Color::Black as usize],
            sum_terms(self.space)
        );
        println!(
            "Threats        | {:^5} | {:^5} | {:^5} |",
            self.threats[Color::White as usize],
            self.threats[Color::Black as usize],
            sum_terms(self.threats)
        );
        println!(
            "Tempo          | {:^5} | {:^5} | {:^5} |",
            self.tempo[Color::White as usize],
            self.tempo[Color::Black as usize],
            sum_terms(self.tempo)
        );
        println!("----------------------------------------");
        println!("Final Score: {}", cp);
        println!("----------------------------------------");
        println!("Remarks");
        println!("----------------------------------------");
        for (square, remark) in &self.remarks {
            println!("Square {}: {}", square, remark);
        }
    }

    #[cfg(not(feature = "trace-eval"))]
    fn dump_evaluation(&self, _: i16) {}
}

fn sum_terms(terms: [i16; 2]) -> i16 {
    terms[Color::White as usize] - terms[Color::Black as usize]
}

pub fn evaluate(pos: &Position) -> Value {
    Evaluator::new(pos).evaluate()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{eval::Value, position::Position};

    #[test]
    fn white_mate_evaluation() {
        let pos = Position::from_fen("8/8/8/8/8/3k4/3q4/3K4 w - - 0 1").unwrap();
        assert_eq!(Value::mated_in(0), evaluate(&pos));
    }

    #[test]
    fn black_mate_evaluation() {
        let pos = Position::from_fen("4k3/4Q3/4K3/8/8/8/8/8 b - - 0 1").unwrap();
        assert_eq!(Value::mate_in(0), evaluate(&pos));
    }

    #[test]
    fn drawn_by_insufficient_material_1() {
        let pos = Position::from_fen("3k4/8/8/8/2N5/8/8/3K4 w - - 0 1").unwrap();
        assert_eq!(Value::new(0), evaluate(&pos));
    }

    #[test]
    fn drawn_by_insufficient_material_2() {
        let pos = Position::from_fen("3k4/8/8/5B2/8/8/8/3K4 w - - 0 1").unwrap();
        assert_eq!(Value::new(0), evaluate(&pos));
    }

    #[test]
    fn drawn_by_insufficient_material_3() {
        let pos = Position::from_fen("3k4/8/8/8/5N2/2N5/8/3K4 w - - 0 1").unwrap();
        assert_eq!(Value::new(0), evaluate(&pos));
    }

    #[test]
    fn drawn_by_insufficient_material_4() {
        let pos = Position::from_fen("3k4/8/8/8/8/8/8/3K4 w - - 0 1").unwrap();
        assert_eq!(Value::new(0), evaluate(&pos));
    }
}
