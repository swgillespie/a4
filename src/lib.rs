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

#![feature(
    try_blocks,
    once_cell,
    bench_black_box,
    core_intrinsics,
    slice_swap_unchecked
)]
#![allow(unused_macros)]

/// Helper macro for writing UCI messages to standard out. This macro echoes the message to standard out while also
/// logging it.
macro_rules! uci_output {
    ($fmt:expr) => {
        {
            always!("uci => {}", format_args!($fmt));
            println!($fmt)
        }
    };
    ($fmt:expr, $($arg:tt)*) => {
        {
            always!("uci => {}", format_args!($fmt, $($arg)*));
            println!($fmt, $($arg)*)
        }
    };
}

macro_rules! log {
    ($level:expr, $format:literal) => {
        crate::log::log($level, format_args!($format));
    };

    ($level:expr, $format:literal, $($args:tt)*) => {
        crate::log::log($level, format_args!($format, $($args)*))
    };
}

macro_rules! debug {
    ($format:literal) => {
        log!(crate::log::LogLevel::Debug, $format);
    };

    ($format:literal, $($args:tt)*) => {
        log!(crate::log::LogLevel::Debug, $format, $($args)*)
    };
}

macro_rules! info {
    ($format:literal) => {
        log!(crate::log::LogLevel::Info, $format);
    };

    ($format:literal, $($args:tt)*) => {
        log!(crate::log::LogLevel::Info, $format, $($args)*)
    };
}

macro_rules! warn {
    ($format:literal) => {
        log!(crate::log::LogLevel::Warn, $format);
    };

    ($format:literal, $($args:tt)*) => {
        log!(crate::log::LogLevel::Warn, $format, $($args)*)
    };
}

macro_rules! always {
    ($format:literal) => {
        log!(crate::log::LogLevel::Always, $format);
    };

    ($format:literal, $($args:tt)*) => {
        log!(crate::log::LogLevel::Always, $format, $($args)*)
    };
}

pub mod core;
pub mod debug;
pub mod eval;
mod log;
pub mod movegen;
pub mod position;
pub mod search;
mod table;
mod threads;
pub mod uci;
mod zobrist;
