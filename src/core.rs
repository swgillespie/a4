// Copyright 2021 Sean Gillespie.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Module `core` contains core datatypes and data structures used pervasively throughout `gambit`.

mod attacks;
mod r#move;
mod squareset;
mod types;

pub use squareset::{SquareSet, SquareSetIterator};
pub use types::{
    colors, files, piece_kinds, ranks, squares, AllFiles, AllRanks, AllSquares, CastleStatus,
    Color, Direction, File, Piece, PieceKind, PieceParseError, Rank, Square, SquareParseError,
};

pub use types::{
    A1, A2, A3, A4, A5, A6, A7, A8, B1, B2, B3, B4, B5, B6, B7, B8, C1, C2, C3, C4, C5, C6, C7, C8,
    D1, D2, D3, D4, D5, D6, D7, D8, E1, E2, E3, E4, E5, E6, E7, E8, F1, F2, F3, F4, F5, F6, F7, F8,
    G1, G2, G3, G4, G5, G6, G7, G8, H1, H2, H3, H4, H5, H6, H7, H8,
};

pub use squareset::{
    SS_FILE_A, SS_FILE_B, SS_FILE_C, SS_FILE_D, SS_FILE_E, SS_FILE_F, SS_FILE_G, SS_FILE_H,
    SS_RANK_1, SS_RANK_2, SS_RANK_3, SS_RANK_4, SS_RANK_5, SS_RANK_6, SS_RANK_7, SS_RANK_8,
};
pub use types::{FILE_A, FILE_B, FILE_C, FILE_D, FILE_E, FILE_F, FILE_G, FILE_H};
pub use types::{RANK_1, RANK_2, RANK_3, RANK_4, RANK_5, RANK_6, RANK_7, RANK_8};

pub use r#move::Move;

pub use attacks::{king_attacks, pawn_attacks};
