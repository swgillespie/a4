// Copyright 2017-2021 Sean Gillespie.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
use crate::core::Move;
use crate::Position;

pub struct ScoredMove(Move, i32);

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum MoveGenMode {
    Quiet,
}
fn generate<const MODE: MoveGenMode>(pos: &Position, moves: &mut Vec<ScoredMove>) {}
