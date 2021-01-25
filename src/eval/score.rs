// Copyright 2021 Sean Gillespie.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
use std::ops;

/// A Value is the static value given to a position by evaluation of the game board. It is a single number, in
/// centipawns, that represents the engine's assessment of a particular position. The number is positive if the engine
/// is winning and negative if the engine is losing.
///
/// In addition to encoding numeric scores, Value also encodes whether or not a checkmate is imminent and, if so, how
/// far it is away.
pub struct Value(i16);

impl ops::Add<Value> for Value {
    type Output = Value;

    fn add(self, rhs: Self) -> Self::Output {
        Value(self.0.saturating_add(rhs.0))
    }
}

impl ops::Add<i16> for Value {
    type Output = Value;

    fn add(self, rhs: i16) -> Self::Output {
        Value(self.0.saturating_add(rhs))
    }
}

impl ops::Neg for Value {
    type Output = Value;

    fn neg(self) -> Self::Output {
        Value(self.0.saturating_neg())
    }
}

impl ops::Mul<i16> for Value {
    type Output = Value;

    fn mul(self, rhs: i16) -> Self::Output {
        Value(self.0.saturating_mul(rhs))
    }
}
