// Copyright 2022 Sean Gillespie.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! A collection of debug utilities that are either executable directly from a debugger or invoke the debugger
//! throughout the course of execution.
use std::{ffi::CString, hint::black_box, ptr};

use crate::{core::Move, eval::Value, position::Position};

#[no_mangle]
pub extern "C" fn pos_str(pos: *const Position) {
    // assumption: `pos` was derived from a Rust reference and thus is not null.
    let pos = unsafe { &*pos };
    print!("{}", pos);
}

#[no_mangle]
pub extern "C" fn pos_fen(pos: *const Position) -> *const i8 {
    // assumption: `pos` was derived from a Rust reference and thus is not null.
    let pos = unsafe { &*pos };
    let body = CString::new(format!("{}", pos.as_fen())).unwrap();
    let leaked = Box::leak(body.into_boxed_c_str());
    return leaked.as_ptr();
}

#[no_mangle]
pub extern "C" fn value_str(value: Value) -> *const i8 {
    let body = CString::new(format!("{:?}", value)).unwrap();
    let leaked = Box::leak(body.into_boxed_c_str());
    return leaked.as_ptr();
}

#[no_mangle]
pub extern "C" fn move_str(mov: Move) -> *const i8 {
    let body = CString::new(format!("{}", mov.as_uci())).unwrap();
    let leaked = Box::leak(body.into_boxed_c_str());
    return leaked.as_ptr();
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
