// Copyright 2021 Sean Gillespie.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
use std::fmt;
use std::i16;
use std::ops;

const VALUE_MATED: i16 = i16::MIN / 2;
const VALUE_MATE: i16 = i16::MAX / 2;

/// A Value is the static value given to a position by evaluation of the game board. It is a single number, in
/// centipawns, that represents the engine's assessment of a particular position. The number is positive if the engine
/// is winning and negative if the engine is losing.
///
/// In addition to encoding numeric scores, Value also encodes whether or not a checkmate is imminent and, if so, how
/// far it is away.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Value(i16);

impl Value {
    pub fn mate_in(ply: i16) -> Value {
        Value(VALUE_MATE + ply)
    }

    pub fn mated_in(ply: i16) -> Value {
        Value(VALUE_MATED - ply)
    }

    pub fn new(evaluation: i16) -> Value {
        Value(evaluation)
    }
}

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

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            v if v > VALUE_MATE => write!(f, "#{}", v - VALUE_MATE),
            v if v < VALUE_MATED => write!(f, "#-{}", -(VALUE_MATED - v)),
            v => write!(f, "{}", v),
        }
    }
}
