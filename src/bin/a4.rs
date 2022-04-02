// Copyright 2021 Sean Gillespie.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use a4::uci;
use tracing_subscriber::{filter::LevelFilter, EnvFilter, FmtSubscriber};

fn main() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(LevelFilter::INFO)
        .with_env_filter(EnvFilter::from_env("A4_LOG"))
        .with_writer(std::io::stderr)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    uci::run().expect("fatal error while running UCI server");
}
