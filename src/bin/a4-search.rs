// Copyright 2022 Sean Gillespie.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
use std::time::Duration;

use a4::{
    position::Position,
    search::{self, SearchOptions},
};
use structopt::StructOpt;

/// Shortcut program for debugging a4's search routines.
#[derive(Debug, StructOpt)]
struct Options {
    /// FEN representation of the position to analyze.
    #[structopt(name = "FEN")]
    fen: String,

    /// Maximum amount of time to spend searching, in seconds.
    #[structopt(short, long)]
    time_sec: Option<u64>,
    /// Maximum number of nodes to search.
    #[structopt(short, long)]
    nodes: Option<u64>,
    /// Maximum depth to search to with a non-specialized search.
    #[structopt(short, long, default_value = "6")]
    depth: u32,
}

fn main() {
    a4::debug::link_in_debug_utils();
    let args = Options::from_args();
    let mut search_options = SearchOptions::default();
    if let Some(time_sec) = args.time_sec {
        let duration = Duration::from_secs(time_sec);
        search_options.time_limit = Some(duration);
    }

    if let Some(nodes) = args.nodes {
        search_options.node_limit = Some(nodes);
    }

    search_options.depth = args.depth;
    let pos = Position::from_fen(args.fen).expect("invalid fen");
    let result = search::search(&pos, &search_options);
    println!("===========================");
    print!("{}", pos);
    println!("===========================");
    println!("{:<15} {}", "Best Move:", result.best_move.as_uci());
    println!("{:<15} {:?}", "Best Score:", result.best_score);
    for (i, depth) in result.stats.nodes_searched_per_depth.iter().enumerate() {
        println!("{:<15} {}", format!("Depth {}:", i + 1), depth);
        if i == 0 {
            continue;
        }

        if let Some(prev) = result.stats.nodes_searched_per_depth.get(i - 1) {
            println!("  {:<5} {}", "EBB:", *depth as f64 / *prev as f64);
        }
    }
}
