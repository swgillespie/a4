// Copyright 2021 Sean Gillespie.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
use crate::core::*;
use crate::eval::{evaluate, Value};
use crate::Position;

struct Searcher {
    nodes_evaluated: u32,
}

pub struct SearchResult {}

impl Searcher {
    fn new() -> Searcher {
        Searcher { nodes_evaluated: 0 }
    }

    fn search(&mut self, pos: &Position, depth: u32) -> SearchResult {
        unimplemented!()
    }

    fn alpha_beta(&mut self, pos: &Position, alpha: Value, beta: Value, depth: u32) -> Value {
        unimplemented!()
    }

    fn quiesce(&mut self, pos: &Position, alpha: Value, beta: Value) -> Value {
        self.nodes_evaluated += 1;
        let value = evaluate(pos);
        if pos.side_to_move() == Color::Black {
            -value
        } else {
            value
        }
    }
}

pub fn search(pos: &Position, depth: u32) -> SearchResult {
    Searcher::new().search(pos, depth)
}
