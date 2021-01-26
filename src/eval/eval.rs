// Copyright 2021 Sean Gillespie.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
use crate::core::*;
use crate::eval::analysis::Analysis;
use crate::eval::Value;
use crate::position::Position;

const KING_WEIGHT: i16 = 10000;
const QUEEN_WEIGHT: i16 = 900;
const ROOK_WEIGHT: i16 = 500;
const BISHOP_WEIGHT: i16 = 300;
const KNIGHT_WEIGHT: i16 = 300;
const PAWN_WEIGHT: i16 = 100;
const PAWN_FORMATION_WEIGHT: i16 = 50;
const MOBILITY_WEIGHT: i16 = 10;

pub fn evaluate(pos: &Position) -> Value {
    let analysis = Analysis::new(pos);

    // Check out mobility first - it's possible that a side has been checkmated.
    let white_mobility = analysis.mobility(Color::White);
    if white_mobility == 0 {
        if pos.is_check(Color::White) {
            return Value::mated_in(0);
        } else {
            return Value::new(0);
        }
    }
    let black_mobility = analysis.mobility(Color::Black);
    if black_mobility == 0 {
        if pos.is_check(Color::Black) {
            return Value::mate_in(0);
        } else {
            return Value::new(0);
        }
    }

    let (kings_w, kings_b) = evaluate_metric(
        KING_WEIGHT,
        |c| if pos.king(c).is_some() { 1 } else { 0 } as i16,
    );

    let (queens_w, queens_b) = evaluate_metric(QUEEN_WEIGHT, |c| pos.queens(c).len() as i16);
    let (rooks_w, rooks_b) = evaluate_metric(ROOK_WEIGHT, |c| pos.rooks(c).len() as i16);
    let (bishops_w, bishops_b) = evaluate_metric(BISHOP_WEIGHT, |c| pos.bishops(c).len() as i16);
    let (knights_w, knights_b) = evaluate_metric(KNIGHT_WEIGHT, |c| pos.knights(c).len() as i16);
    let (pawns_w, pawns_b) = evaluate_metric(PAWN_WEIGHT, |c| pos.pawns(c).len() as i16);
    let mobility = MOBILITY_WEIGHT * (white_mobility as i16 - black_mobility as i16);
    let (isolated_pawns_w, isolated_pawns_b) = evaluate_metric(PAWN_FORMATION_WEIGHT, |c| {
        analysis.isolated_pawns(c).len() as i16
    });
    let (backward_pawns_w, backward_pawns_b) = evaluate_metric(PAWN_FORMATION_WEIGHT, |c| {
        analysis.backward_pawns(c).len() as i16
    });
    let (doubled_pawns_w, doubled_pawns_b) = evaluate_metric(PAWN_FORMATION_WEIGHT, |c| {
        analysis.doubled_pawns(c).len() as i16
    });

    let value = (kings_w - kings_b)
        + (queens_w - queens_b)
        + (rooks_w - rooks_b)
        + (bishops_w - bishops_b)
        + (knights_w - knights_b)
        + (pawns_w - pawns_b)
        + (isolated_pawns_w - isolated_pawns_b)
        + (backward_pawns_w - backward_pawns_b)
        + (doubled_pawns_w - doubled_pawns_b)
        + mobility;

    #[cfg(feature = "trace-eval")]
    {
        println!("========================================");
        println!("FEN: {}", pos.as_fen());
        println!("========================================");
        println!("Term           | White | Black | Total |");
        println!("----------------------------------------");
        println!(
            "Mobility       | {:^5} | {:^5} | {:^5} |",
            MOBILITY_WEIGHT * white_mobility as i16,
            MOBILITY_WEIGHT * black_mobility as i16,
            mobility
        );
        println!(
            "Kings          | {:^5} | {:^5} | {:^5} |",
            kings_w,
            kings_b,
            (kings_w - kings_b)
        );
        println!(
            "Queens         | {:^5} | {:^5} | {:^5} |",
            queens_w,
            queens_b,
            (queens_w - queens_b)
        );
        println!(
            "Rooks          | {:^5} | {:^5} | {:^5} |",
            rooks_w,
            rooks_b,
            (rooks_w - rooks_b)
        );
        println!(
            "Bishops        | {:^5} | {:^5} | {:^5} |",
            bishops_w,
            bishops_b,
            (bishops_w - bishops_b)
        );
        println!(
            "Knights        | {:^5} | {:^5} | {:^5} |",
            knights_w,
            knights_b,
            (knights_w - knights_b)
        );
        println!(
            "Pawns          | {:^5} | {:^5} | {:^5} |",
            pawns_w,
            pawns_b,
            (pawns_w - pawns_b)
        );
        println!(
            "Isolated Pawns | {:^5} | {:^5} | {:^5} |",
            isolated_pawns_w,
            isolated_pawns_b,
            (isolated_pawns_w - isolated_pawns_b)
        );
        println!(
            "Backward Pawns | {:^5} | {:^5} | {:^5} |",
            backward_pawns_w,
            backward_pawns_b,
            (backward_pawns_w - backward_pawns_b)
        );
        println!(
            "Doubled Pawns  | {:^5} | {:^5} | {:^5} |",
            doubled_pawns_w,
            doubled_pawns_b,
            (doubled_pawns_w - doubled_pawns_b)
        );

        println!("========================================");
        println!("Total: {}", value)
    }

    Value::new(value)
}

fn evaluate_metric<F>(weight: i16, func: F) -> (i16, i16)
where
    F: Fn(Color) -> i16,
{
    (weight * func(Color::White), weight * func(Color::Black))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::Value;
    use crate::position::Position;

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
