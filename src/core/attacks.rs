// Copyright 2017-2021 Sean Gillespie.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
use std::sync::LazyLock;

use crate::core::*;

const SS_RANK_12: SquareSet = SS_RANK_1.or(SS_RANK_2);
const SS_RANK_78: SquareSet = SS_RANK_7.or(SS_RANK_8);

const SS_FILE_AB: SquareSet = SS_FILE_A.or(SS_FILE_B);
const SS_FILE_GH: SquareSet = SS_FILE_G.or(SS_FILE_H);

struct KingTable {
    table: [SquareSet; 64],
}

impl KingTable {
    pub fn new() -> KingTable {
        let mut kt = KingTable {
            table: [SquareSet::empty(); 64],
        };

        for sq in squares() {
            let mut board = SquareSet::empty();
            if !SS_RANK_8.contains(sq) {
                board.insert(sq.plus(8));
                if !SS_FILE_A.contains(sq) {
                    board.insert(sq.plus(7));
                }
                if !SS_FILE_H.contains(sq) {
                    board.insert(sq.plus(9));
                }
            }

            if !SS_RANK_1.contains(sq) {
                board.insert(sq.plus(-8));
                if !SS_FILE_A.contains(sq) {
                    board.insert(sq.plus(-9));
                }
                if !SS_FILE_H.contains(sq) {
                    board.insert(sq.plus(-7));
                }
            }

            if !SS_FILE_A.contains(sq) {
                board.insert(sq.plus(-1));
            }
            if !SS_FILE_H.contains(sq) {
                board.insert(sq.plus(1));
            }

            kt.table[sq.0 as usize] = board;
        }

        kt
    }

    pub fn attacks(&self, sq: Square) -> SquareSet {
        self.table[sq.0 as usize]
    }
}

struct PawnTable {
    table: [[SquareSet; 2]; 64],
}

impl PawnTable {
    pub fn new() -> PawnTable {
        let mut pt = PawnTable {
            table: [[SquareSet::empty(); 2]; 64],
        };

        for sq in squares() {
            for color in colors() {
                let mut board = SquareSet::empty();
                let (promo_rank, up_left, up_right) = match color {
                    Color::White => (SS_RANK_8, 7, 9),
                    Color::Black => (SS_RANK_1, -9, -7),
                };

                if promo_rank.contains(sq) {
                    // No legal moves for this particular pawn. It's generally impossible
                    // for pawns to be on the promotion rank anyway since they should have
                    // been promoted already.
                    continue;
                }

                if !SS_FILE_A.contains(sq) {
                    board.insert(sq.plus(up_left));
                }
                if !SS_FILE_H.contains(sq) {
                    board.insert(sq.plus(up_right));
                }

                pt.table[sq.0 as usize][color as usize] = board;
            }
        }

        pt
    }

    pub fn attacks(&self, sq: Square, color: Color) -> SquareSet {
        self.table[sq.0 as usize][color as usize]
    }
}

struct KnightTable {
    table: [SquareSet; 64],
}

impl KnightTable {
    pub fn new() -> KnightTable {
        let mut kt = KnightTable {
            table: [SquareSet::empty(); 64],
        };

        for sq in squares() {
            let mut board = SquareSet::empty();
            if !SS_FILE_A.contains(sq) && !SS_RANK_78.contains(sq) {
                board.insert(sq.plus(15));
            }
            if !SS_FILE_H.contains(sq) && !SS_RANK_78.contains(sq) {
                board.insert(sq.plus(17));
            }
            if !SS_FILE_GH.contains(sq) && !SS_RANK_8.contains(sq) {
                board.insert(sq.plus(10));
            }
            if !SS_FILE_GH.contains(sq) && !SS_RANK_1.contains(sq) {
                board.insert(sq.plus(-6));
            }
            if !SS_FILE_H.contains(sq) && !SS_RANK_12.contains(sq) {
                board.insert(sq.plus(-15));
            }
            if !SS_FILE_A.contains(sq) && !SS_RANK_12.contains(sq) {
                board.insert(sq.plus(-17));
            }
            if !SS_FILE_AB.contains(sq) && !SS_RANK_1.contains(sq) {
                board.insert(sq.plus(-10));
            }
            if !SS_FILE_AB.contains(sq) && !SS_RANK_8.contains(sq) {
                board.insert(sq.plus(6));
            }
            kt.table[sq.0 as usize] = board;
        }
        kt
    }

    pub fn attacks(&self, sq: Square) -> SquareSet {
        self.table[sq.0 as usize]
    }
}

struct RayTable {
    table: [[SquareSet; 8]; 65],
}

impl RayTable {
    pub fn new() -> RayTable {
        let mut rt = RayTable {
            table: [[SquareSet::empty(); 8]; 65],
        };

        for sq in squares() {
            let mut populate_dir = |dir: Direction, edge: SquareSet| {
                let mut entry = SquareSet::empty();
                if edge.contains(sq) {
                    // Nothing to do here, there are no legal moves on this ray from this square.
                    rt.table[sq.0 as usize][dir as usize] = entry;
                    return;
                }

                // Starting at the given square, cast a ray in the given direction and add all bits to the ray mask.
                let mut cursor = sq;
                loop {
                    cursor = cursor.towards(dir);
                    entry.insert(cursor);

                    // Did we reach the end of the board? If so, stop.
                    if edge.contains(cursor) {
                        break;
                    }
                }
                rt.table[sq.0 as usize][dir as usize] = entry;
            };

            populate_dir(Direction::North, SS_RANK_8);
            populate_dir(Direction::NorthEast, SS_RANK_8.or(SS_FILE_H));
            populate_dir(Direction::East, SS_FILE_H);
            populate_dir(Direction::SouthEast, SS_RANK_1.or(SS_FILE_H));
            populate_dir(Direction::South, SS_RANK_1);
            populate_dir(Direction::SouthWest, SS_RANK_1.or(SS_FILE_A));
            populate_dir(Direction::West, SS_FILE_A);
            populate_dir(Direction::NorthWest, SS_RANK_8.or(SS_FILE_A));
        }
        rt
    }

    pub fn attacks(&self, sq: usize, dir: Direction) -> SquareSet {
        self.table[sq as usize][dir as usize]
    }
}

static KING_TABLE: LazyLock<KingTable> = LazyLock::new(KingTable::new);
static PAWN_TABLE: LazyLock<PawnTable> = LazyLock::new(PawnTable::new);
static KNIGHT_TABLE: LazyLock<KnightTable> = LazyLock::new(KnightTable::new);
static RAY_TABLE: LazyLock<RayTable> = LazyLock::new(RayTable::new);

fn positive_ray_attacks(sq: Square, occupancy: SquareSet, dir: Direction) -> SquareSet {
    debug_assert!(dir.as_vector() > 0);
    let attacks = RAY_TABLE.attacks(sq.0 as usize, dir);
    let blocker = attacks.and(occupancy).bits();
    let blocking_square = blocker.trailing_zeros() as usize;
    let blocking_ray = RAY_TABLE.attacks(blocking_square, dir);
    attacks.xor(blocking_ray)
}

fn negative_ray_attacks(sq: Square, occupancy: SquareSet, dir: Direction) -> SquareSet {
    debug_assert!(dir.as_vector() < 0);
    let attacks = RAY_TABLE.attacks(sq.0 as usize, dir);
    let blocker = attacks.and(occupancy).bits();
    let blocking_square = (64 - blocker.leading_zeros()).checked_sub(1).unwrap_or(64) as usize;
    let blocking_ray = RAY_TABLE.attacks(blocking_square, dir);
    attacks.xor(blocking_ray)
}

fn diagonal_attacks(sq: Square, occupancy: SquareSet) -> SquareSet {
    positive_ray_attacks(sq, occupancy, Direction::NorthWest)
        | negative_ray_attacks(sq, occupancy, Direction::SouthEast)
}

fn antidiagonal_attacks(sq: Square, occupancy: SquareSet) -> SquareSet {
    positive_ray_attacks(sq, occupancy, Direction::NorthEast)
        | negative_ray_attacks(sq, occupancy, Direction::SouthWest)
}

fn file_attacks(sq: Square, occupancy: SquareSet) -> SquareSet {
    positive_ray_attacks(sq, occupancy, Direction::North)
        | negative_ray_attacks(sq, occupancy, Direction::South)
}

fn rank_attacks(sq: Square, occupancy: SquareSet) -> SquareSet {
    positive_ray_attacks(sq, occupancy, Direction::East)
        | negative_ray_attacks(sq, occupancy, Direction::West)
}

pub fn pawn_attacks(sq: Square, color: Color) -> SquareSet {
    PAWN_TABLE.attacks(sq, color)
}

pub fn bishop_attacks(sq: Square, occupancy: SquareSet) -> SquareSet {
    diagonal_attacks(sq, occupancy) | antidiagonal_attacks(sq, occupancy)
}

pub fn knight_attacks(sq: Square) -> SquareSet {
    KNIGHT_TABLE.attacks(sq)
}

pub fn rook_attacks(sq: Square, occupancy: SquareSet) -> SquareSet {
    file_attacks(sq, occupancy) | rank_attacks(sq, occupancy)
}

pub fn queen_attacks(sq: Square, occupancy: SquareSet) -> SquareSet {
    bishop_attacks(sq, occupancy) | rook_attacks(sq, occupancy)
}

pub fn king_attacks(sq: Square) -> SquareSet {
    KING_TABLE.attacks(sq)
}

pub fn attacks(kind: PieceKind, color: Color, sq: Square, occupancy: SquareSet) -> SquareSet {
    match kind {
        PieceKind::Pawn => pawn_attacks(sq, color),
        PieceKind::Knight => knight_attacks(sq),
        PieceKind::Bishop => bishop_attacks(sq, occupancy),
        PieceKind::Rook => rook_attacks(sq, occupancy),
        PieceKind::Queen => queen_attacks(sq, occupancy),
        PieceKind::King => king_attacks(sq),
    }
}
