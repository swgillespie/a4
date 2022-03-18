// Copyright 2021 Sean Gillespie.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
use std::{fmt, i16, ops};

const VALUE_MATED: i16 = i16::MIN / 2 + 1;
const VALUE_MATE: i16 = i16::MAX / 2;
const MATE_DISTANCE_MAX: i16 = 50;

/// A Value is the static value given to a position by evaluation of the game board. It is a single number, in
/// centipawns, that represents the engine's assessment of a particular position. The number is positive if the engine
/// is winning and negative if the engine is losing.
///
/// In addition to encoding numeric scores, Value also encodes whether or not a checkmate is imminent and, if so, how
/// far it is away.
///
/// # Representation
/// The Value structure makes use of the range of i16 to encode centipawn scores. Two key constants form the boundary
/// of valid scores:
///   1. VALUE_MATED: `i16::MIN/2 + 1` (-16383)
///   2. VALUE_MATE: `i16::MAX/2` (16383)
///
/// Because of this constrained value, we must take care that the addition or subtraction of scores do not cross these
/// thresholds. This check is dynamic.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Value(i16);

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum UnpackedValue {
    MateIn(u16),
    MatedIn(u16),
    Value(i16),
}

impl Value {
    pub fn mate_in(ply: i16) -> Value {
        debug_assert!(ply < MATE_DISTANCE_MAX);
        Value(VALUE_MATE + MATE_DISTANCE_MAX - ply)
    }

    pub fn mated_in(ply: i16) -> Value {
        debug_assert!(ply < MATE_DISTANCE_MAX);
        Value(VALUE_MATED - MATE_DISTANCE_MAX + ply)
    }

    pub fn new(evaluation: i16) -> Value {
        Value(evaluation)
    }

    pub fn step(self) -> Value {
        match self.unpack() {
            UnpackedValue::MateIn(value) => Value::mate_in((value + 1) as i16),
            UnpackedValue::MatedIn(value) => Value::mated_in((value + 1) as i16),
            _ => self,
        }
    }

    /// Unpacks a Value from its efficient representation to a matchable representation.
    pub fn unpack(self) -> UnpackedValue {
        match self.0 {
            v if v > VALUE_MATE => {
                UnpackedValue::MateIn((VALUE_MATE + MATE_DISTANCE_MAX - v) as u16)
            }
            v if v < VALUE_MATED => {
                UnpackedValue::MatedIn((v - VALUE_MATED + MATE_DISTANCE_MAX) as u16)
            }
            v => UnpackedValue::Value(v),
        }
    }

    /// Formats this value in a format understood by UCI.
    pub fn as_uci(self) -> String {
        match self.unpack() {
            UnpackedValue::MateIn(moves) => {
                format!("mate {}", moves)
            }
            UnpackedValue::MatedIn(moves) => {
                format!("mate -{}", moves)
            }
            UnpackedValue::Value(value) => {
                format!("cp {}", value)
            }
        }
    }

    fn add(self, other: Value) -> Value {
        debug_assert!(self.0 > VALUE_MATED && self.0 < VALUE_MATE);
        let mut next = self.0 + other.0;
        if next <= VALUE_MATED || next >= VALUE_MATE {
            if next <= VALUE_MATED {
                next = VALUE_MATED + 1;
            } else {
                next = VALUE_MATE - 1;
            }
        }
        Value(next)
    }
}

impl ops::Add<Value> for Value {
    type Output = Value;

    fn add(self, rhs: Self) -> Self::Output {
        self.add(rhs)
    }
}

impl ops::Add<i16> for Value {
    type Output = Value;

    fn add(self, rhs: i16) -> Self::Output {
        debug_assert!(rhs > VALUE_MATED && rhs < VALUE_MATE);
        self.add(Value(rhs))
    }
}

impl ops::Sub<Value> for Value {
    type Output = Value;

    fn sub(self, rhs: Value) -> Self::Output {
        self.add(-rhs)
    }
}

impl ops::Sub<i16> for Value {
    type Output = Value;

    fn sub(self, rhs: i16) -> Self::Output {
        debug_assert!(rhs > VALUE_MATED && rhs < VALUE_MATE);
        self.add(Value(-rhs))
    }
}

impl ops::Neg for Value {
    type Output = Value;

    fn neg(self) -> Self::Output {
        Value(self.0.saturating_neg())
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.unpack() {
            UnpackedValue::MateIn(moves) => write!(f, "#{}", moves),
            UnpackedValue::MatedIn(moves) => write!(f, "#-{}", moves),
            UnpackedValue::Value(value) => write!(f, "{}", value),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[cfg(test)]
mod tests {
    use super::{Value, VALUE_MATE, VALUE_MATED};
    use crate::eval::UnpackedValue;

    #[test]
    fn value_negate() {
        let v = Value::mate_in(4);
        assert_eq!(-v, Value::mated_in(4));
    }

    #[test]
    fn value_saturating_add() {
        let mut v = Value::new(VALUE_MATE - 1);
        v = v + 3;
        assert_eq!(v.0, VALUE_MATE - 1);
    }

    #[test]
    fn value_saturating_sub() {
        let mut v = Value::new(VALUE_MATED + 1);
        v = v - 3;
        assert_eq!(v.0, VALUE_MATED + 1);
    }

    #[test]
    fn mated_in_4_is_better_than_mated_in_3() {
        assert!(Value::mated_in(4) > Value::mated_in(3))
    }

    #[test]
    fn mate_in_2_is_worse_than_mate_in_1() {
        assert!(Value::mate_in(2) < Value::mate_in(1))
    }

    #[test]
    fn unpack_mated_in() {
        let mated_in_one = Value::mated_in(1);
        assert_eq!(mated_in_one.unpack(), UnpackedValue::MatedIn(1));
    }

    #[test]
    fn unpack_mate_in() {
        let mate_in_one = Value::mate_in(1);
        assert_eq!(mate_in_one.unpack(), UnpackedValue::MateIn(1));
    }
}
