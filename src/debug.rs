// Copyright 2022 Sean Gillespie.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! A collection of debug utilities that are either executable directly from a debugger or invoke the debugger
//! throughout the course of execution.
use std::{hint::black_box, ptr};

use crate::{core::Move, eval::Value, position::Position};

#[no_mangle]
pub extern "C" fn pos_str(pos: *const Position) {
    // assumption: `pos` was derived from a Rust reference and thus is not null.
    let pos = unsafe { &*pos };
    print!("{}", pos);
}

#[no_mangle]
pub extern "C" fn pos_fen(pos: *const Position) {
    // assumption: `pos` was derived from a Rust reference and thus is not null.
    let pos = unsafe { &*pos };
    println!("{}", pos.as_fen());
}

#[no_mangle]
pub extern "C" fn value_str(value: Value) {
    println!("{:?}", value);
}

#[no_mangle]
pub extern "C" fn move_str(mov: Move) {
    println!("{}", mov.as_uci());
}

#[no_mangle]
pub extern "C" fn breakpoint() {
    unsafe {
        std::intrinsics::breakpoint();
    }
}

/// The `no_mangle` attribute does not force binaries to link in these symbols; this function does, if it is called
/// from a binary. Calling this function does nothing at runtime.
pub fn link_in_debug_utils() {
    if black_box(false) {
        pos_str(ptr::null());
        pos_fen(ptr::null());
        value_str(Value::mate_in(1));
        move_str(Move::null());
        breakpoint();
    }
}
