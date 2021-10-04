// Copyright 2021 Sean Gillespie.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! An implementation of the UCI protocol for a4, driving our internal search routines.
//! See [here](http://wbec-ridderkerk.nl/html/UCIProtocol.html) for full documentation on the protocol.

use crate::{core::Move, threads, threads::SearchRequest, Position};
use anyhow::anyhow;
use std::{
    io::{self, BufRead},
    time::Duration,
};

pub fn run() -> io::Result<()> {
    threads::initialize();
    let stdin = io::stdin();
    let locked_stdin = stdin.lock();
    for maybe_line in locked_stdin.lines() {
        let line = maybe_line?;
        let components: Vec<_> = line.split_whitespace().collect();
        let (&command, arguments) = components.split_first().unwrap_or((&"", &[]));
        match (command, arguments) {
            ("uci", []) => handle_uci(),
            ("debug", ["on"]) => {}
            ("debug", ["off"]) => {}
            ("isready", []) => handle_isready(),
            ("ucinewgame", []) => handle_ucinewgame(),
            ("position", args) => handle_position(args),
            ("go", args) => handle_go(args),
            ("stop", []) => handle_stop(),
            ("quit", []) => return Ok(()),
            _ => println!("unrecognized command: {} {:?}", command, arguments),
        }
    }

    Ok(())
}

fn handle_uci() {
    println!(
        "id name {} {}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    );
    println!("id author {}", env!("CARGO_PKG_AUTHORS"));
    println!("uciok");
}

fn handle_stop() {
    threads::get().main_thread().stop();
}

fn handle_isready() {
    // TODO(swgillespie) ask the main thread if it's idle and all worker threads are idle?
    println!("readyok");
}

fn handle_position(args: &[&str]) {
    let mut position = Position::new();
    let mut iter = args.iter().cloned().peekable();
    let result: anyhow::Result<()> = try {
        loop {
            match iter.next() {
                Some("fen") => {
                    let mut fen_str = Vec::new();
                    while let Some(next) = iter.peek() {
                        if *next == "moves" {
                            break;
                        }

                        let next = iter.next().unwrap();
                        fen_str.push(next.to_owned());
                    }
                    let fen = fen_str.join(" ");
                    position = Position::from_fen(fen)?;
                }
                Some("startpos") => {
                    position = Position::from_start_position();
                }
                Some("moves") => {
                    while let Some(mov_str) = iter.next() {
                        let mov = Move::from_uci(&position, mov_str)
                            .ok_or_else(|| anyhow!("invalid move: {}", mov_str))?;
                        position.make_move(mov);
                    }
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

fn handle_go(args: &[&str]) {
    let mut iter = args.iter().cloned();
    let mut options: SearchRequest = Default::default();
    let result: anyhow::Result<()> = try {
        loop {
            match iter.next() {
                Some("searchmoves") => {
                    // TODO(swgillespie) restricting the initial set of search moves
                }
                Some("ponder") => {
                    // TODO(swgillespie) pondering
                }
                Some("wtime") => {
                    let _time: u64 = iter
                        .next()
                        .ok_or_else(|| anyhow!("expected duration after wtime"))?
                        .parse()?;
                    // TODO(swgillespie) clock management
                }
                Some("btime") => {
                    let _time: u64 = iter
                        .next()
                        .ok_or_else(|| anyhow!("expected duration after btime"))?
                        .parse()?;
                    // TODO(swgillespie) clock management
                }
                Some("winc") => {
                    let _inc: u64 = iter
                        .next()
                        .ok_or_else(|| anyhow!("expected duration after winc"))?
                        .parse()?;
                    // TODO(swgillespie) clock management
                }
                Some("binc") => {
                    let _inc: u64 = iter
                        .next()
                        .ok_or_else(|| anyhow!("expected duration after binc"))?
                        .parse()?;
                    // TODO(swgillespie) clock management
                }
                Some("movestogo") => {
                    let _movestogo: u64 = iter
                        .next()
                        .ok_or_else(|| anyhow!("expected move count after movestogo"))?
                        .parse()?;
                    // TODO(swgillespie) clock management
                }
                Some("depth") => {
                    let maxdepth: u32 = iter
                        .next()
                        .ok_or_else(|| anyhow!("expected move count after movestogo"))?
                        .parse()?;
                    options.depth = Some(maxdepth);
                }
                Some("nodes") => {
                    let nodes: u64 = iter
                        .next()
                        .ok_or_else(|| anyhow!("expected node count after nodes"))?
                        .parse()?;
                    options.node_limit = Some(nodes);
                }
                Some("mate") => {
                    // TODO(swgillespie) mate search
                }
                Some("movetime") => {
                    let msec: u64 = iter
                        .next()
                        .ok_or_else(|| anyhow!("expected msec count after movetime"))?
                        .parse()?;
                    options.time_limit = Some(Duration::from_millis(msec));
                }
                Some("infinite") => {
                    options.time_limit = None;
                }
                Some(tok) => Err(anyhow!("unexpected token: {}", tok))?,
                None => break,
            }
        }
    };

    match result {
        Ok(()) => {
            threads::get().main_thread().search(options);
        }
        Err(e) => println!("invalid go command: {}", e),
    }
}

fn handle_ucinewgame() {
    threads::get().main_thread().set_position(Position::new());
    // TODO(swgillespie) clear transposition tables, when they exist
}
