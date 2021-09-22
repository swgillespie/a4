// Copyright 2021 Sean Gillespie.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
use a4::{search, Position};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
struct Options {
    /// FEN representation of the position to analyze.
    #[structopt(name = "FEN")]
    fen: String,
    /// Depth to which to search
    #[structopt(short, long)]
    depth: u32,
    /// If present, serializes the output as json.
    #[structopt(long)]
    json: bool,
}

fn main() {
    let ops = Options::from_args();
    let pos = Position::from_fen(ops.fen).unwrap();
    let result = search::search(&pos, ops.depth);
    if ops.json {
        println!(
            "{{
    \"best_move\": \"{}\",
    \"best_score\": {},
    \"nodes_evaluated\": {},
}}",
            result.best_move, result.best_score, result.nodes_evaluated
        );
    } else {
        println!("{:?}", result);
    }
}
