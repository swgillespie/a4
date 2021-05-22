// Copyright 2017-2021 Sean Gillespie.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! The `a4` chess engine and library, at your service!
//!
//! `a4` aims to be a one-stop shop for Chess programming in Rust. As a library, `a4` is capable of analyzing
//! positions, reading and writing common Chess formats, and manipulating board positions. As an executable, `a4`
//! is capable of playing chess via the `UCI` protocol.

pub mod core;
pub mod eval;
pub mod movegen;
mod position;
mod search;

pub use position::Position;
