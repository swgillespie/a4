// Copyright 2021 Sean Gillespie.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! An implementation of the UCI protocol for a4, driving our internal search routines.
//! See [here](http://wbec-ridderkerk.nl/html/UCIProtocol.html) for full documentation on the protocol.

use crate::threads;
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
