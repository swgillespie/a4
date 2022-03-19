// Copyright 2017-2022 Sean Gillespie.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Tracing constants that are used elsewhere in a4.
//!
//! The code in `a4::tracing` operates by creating spans and messages with known string values, so that they can be
//! interpreted by `tracing` Layers that are operating upon them. This module collects them all in one place.

/// The name of a span representing a call to `alpha_beta` in Search.
pub const ALPHA_BETA: &'static str = "ab";

/// The name of a span representing a call to `alpha_beta`, particularly to evaluate the fitness of a hash move.
pub const ALPHA_BETA_HASH_MOVE: &'static str = "ab_hash_move";

/// The name of a span representing a call to `alpha_beta`, particularly to evaluate the fitness of a hash move.
pub const ALPHA_BETA_MOVE: &'static str = "ab_move";

pub const Q_SEARCH: &'static str = "qsearch";

pub const Q_SEARCH_MOVE: &'static str = "qsearch_move";

/// A search was prematurely ended, either through a time limit, node limit, or explicit stop.
pub const SEARCH_TERMINATION: &'static str = "explicit search termination";

/// Search hit the t-table when consulting it.
pub const TT_CUTOFF: &'static str = "tt cutoff";

pub const HASH_MOVE_BETA_CUTOFF: &'static str = "hash move beta cutoff";

pub const HASH_MOVE_IMPROVED_ALPHA: &'static str = "hash move beta cutoff";

pub const MOVE_BETA_CUTOFF: &'static str = "move beta cutoff";

pub const MOVE_IMPROVED_ALPHA: &'static str = "move improved alpha";

pub const STAND_PAT_BETA_CUTOFF: &'static str = "stand pat beta cutoff";

pub const STAND_PAT_IMPROVED_ALPHA: &'static str = "stand pat improved alpha";

pub const ALPHA_BETA_ALL: &'static str = "all node";
