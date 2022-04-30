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
const MOBILITY_WEIGHT: i16 = 10;

// Pawn piece modifiers
const ISOLATED_PAWN_MODIFIER: i16 = 17;
const BACKWARD_PAWN_MODIFIER: i16 = 10;
const DOUBLED_PAWN_MODIFIER: i16 = 10;

pub struct Evaluator<'a> {
    analysis: Analysis<'a>,
    mobility: [i16; 2],
    material: [i16; 2],
    pawn_modifiers: [i16; 2],
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

        self.mobility[Color::White as usize] = white_mobility as i16 * MOBILITY_WEIGHT;
        self.mobility[Color::Black as usize] = black_mobility as i16 * MOBILITY_WEIGHT;

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

        self.dump_evaluation();
        let centipawns =
            sum_terms(self.material) + sum_terms(self.mobility) + sum_terms(self.pawn_modifiers);
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

    #[cfg(feature = "trace-eval")]
    fn remark(&mut self, square: Square, remark: &'static str) {
        self.remarks.push((square, remark));
    }

    #[cfg(not(feature = "trace-eval"))]
    fn remark(&mut self, _: Square, _: &'static str) {}

    #[cfg(feature = "trace-eval")]
    fn dump_evaluation(&self) {
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
        println!("----------------------------------------");
        println!("Remarks");
        println!("----------------------------------------");
        for (square, remark) in &self.remarks {
            println!("Square {}: {}", square, remark);
        }
    }

    #[cfg(not(feature = "trace-eval"))]
    fn dump_evaluation(&self) {}
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
}
