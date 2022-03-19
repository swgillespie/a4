// Copyright 2017-2022 Sean Gillespie.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! The `tracing` module interfaces with the `tracing` crate to instrument key parts of a4 for debugging things like
//! search routines that are otherwise difficult to debug.

pub mod constants;
pub mod search;
