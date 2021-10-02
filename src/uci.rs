// Copyright 2021 Sean Gillespie.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! An implementation of the UCI protocol for a4, driving our internal search routines.
//! See [here](http://wbec-ridderkerk.nl/html/UCIProtocol.html) for full documentation on the protocol.

use crate::{threads, Position};
use anyhow::anyhow;
use std::io::{self, BufRead};

pub fn run() -> io::Result<()> {
    let stdin = io::stdin();
    let locked_stdin = stdin.lock();
    for maybe_line in locked_stdin.lines() {
        let line = maybe_line?;
        let components: Vec<_> = line.split_whitespace().collect();
        let (&command, arguments) = components.split_first().unwrap_or((&"", &[]));
        match (command, arguments) {
            ("uci", []) => handle_uci(),
            ("isready", []) => handle_isready(),
            ("position", args) => handle_position(args),
            ("a4_start", []) => handle_a4_start(),
            ("a4_stop", []) => handle_a4_stop(),
            _ => println!("unrecognized command: {} {:?}", command, arguments),
        }
    }

    Ok(())
}

fn handle_uci() {
    threads::initialize();
    println!(
        "id name {} {}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    );
    println!("id author {}", env!("CARGO_PKG_AUTHORS"));
    println!("uciok");
}

fn handle_isready() {
    // TODO(swgillespie) ask the main thread if it's idle and all worker threads are idle?
    println!("readyok");
}

fn handle_position(args: &[&str]) {
    let mut position = Position::new();
    let mut iter = args.iter().cloned();
    let result: anyhow::Result<()> = try {
        loop {
            match iter.next() {
                Some("fen") => {
                    let fen = iter.next().ok_or_else(|| anyhow!("FEN string expected"))?;
                    position = Position::from_fen(fen)?;
                }
                Some("startpos") => {
                    position = Position::from_start_position();
                }
                Some("moves") => {
                    // TODO(swgillespie) parse moves into UCI and apply them to the position
                }
                Some(tok) => {
                    Err(anyhow!("unknown token: {}", tok))?;
                }
                None => break,
            }
        }
    };

    match result {
        Ok(()) => threads::get().main_thread().set_position(position),
        Err(e) => println!("invalid position command: {}", e),
    }
}

// Temporary extensions to UCI to test out our thread harness.

fn handle_a4_start() {
    let threads = threads::get();
    threads.main_thread().search();
}

fn handle_a4_stop() {
    let threads = threads::get();
    threads.main_thread().stop();
}
