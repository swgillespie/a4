// Copyright 2019-2021 Sean Gillespie.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

/// A4's transposition table, which is responsible for memoizing search results
/// for individual positions.
///
/// Despite the combinatorial explosion of possible positions of a Chess board,
/// it is often the case that there are many sequences of moves that lead to
/// the same position on the board. Borrowing from Chess parlance, these are
/// called "transpositions". The purpose of the transposition table is to
/// encode this intuition into a shared memory of positions that have already
/// been seen, but from a different sequence of moves. In a SMP search, this
/// scheme also allows threads to avoid re-calculating a node that has already
/// been evaluated by another thread.
///
/// # Node Kinds
/// The transposition table encodes three kinds of results:
///   * A `pv` node, or "principal variation". A `pv` node has a few useful properties:
///     * All moves have been searched for a `pv` node, and thus the value contained within is exact.
///     * All siblings of a `pv` node are `cut` nodes.
///   * A `cut` node, or "fail-high" node, represents a node in which a beta-cutoff occurred. This means that we found
///     a refutation in this node that is good enough that there is no need to continue the search - this position
///     should not be entered.
///       * A minimum of one move was examined for a `cut` position.
///       * The value stored within is a lower bound on the exact score.
///   * A `all` node, or "fail-low" node, represents a node in which no move's score exceeded the search's alpha. All
///     moves were searched in this position and no move was good enough to exceed the alpha. This implies that a
///     a sibling node is a better move and this node does not need to be searched any deeper.
///
use chashmap::{CHashMap, ReadGuard};
use std::fmt;
use std::lazy::SyncLazy;

use crate::{core::Move, eval::Value, Position};

/// A read-only reference to an entry in the transposition table.
pub struct Entry<'a>(ReadGuard<'a, u64, TableEntry>);

impl<'a> Entry<'a> {
    pub fn best_move(&self) -> Option<Move> {
        self.0.best_move
    }

    pub fn depth(&self) -> u32 {
        self.0.depth
    }

    pub fn kind(&self) -> NodeKind {
        self.0.node
    }
}

impl fmt::Debug for Entry<'static> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Entry")
            .field("best_move", &self.best_move())
            .field("depth", &self.depth())
            .field("kind", &self.kind())
            .finish()
    }
}

struct Table {
    map: CHashMap<u64, TableEntry>,
}

impl Table {
    fn new() -> Table {
        Table {
            map: CHashMap::new(),
        }
    }

    fn record_pv(&self, pos: &Position, best_move: Move, depth: u32, value: Value) {
        let key = pos.zobrist_hash();
        let entry = TableEntry {
            zobrist_key: key,
            best_move: Some(best_move),
            depth,
            node: NodeKind::PV(value),
        };

        self.map.insert(key, entry);
    }

    pub fn record_cut(&self, pos: &Position, best_move: Move, depth: u32, value: Value) {
        let key = pos.zobrist_hash();
        let entry = TableEntry {
            zobrist_key: key,
            best_move: Some(best_move),
            depth,
            node: NodeKind::Cut(value),
        };

        self.map.insert(key, entry);
    }

    pub fn record_all(&self, pos: &Position, depth: u32, value: Value) {
        if let Some(existing) = self.map.get(&pos.zobrist_hash()) {
            if existing.is_all() {
                if existing.depth > depth {
                    return;
                }
            } else {
                return;
            }
        }

        let key = pos.zobrist_hash();
        let entry = TableEntry {
            zobrist_key: key,
            best_move: None,
            depth,
            node: NodeKind::All(value),
        };

        self.map.insert(key, entry);
    }

    pub fn query(&self, pos: &Position) -> Option<Entry<'_>> {
        let key = pos.zobrist_hash();
        self.map.get(&key).map(Entry)
    }

    pub fn clear(&self) {
        self.map.clear();
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum NodeKind {
    PV(Value),
    All(Value),
    Cut(Value),
}

struct TableEntry {
    pub zobrist_key: u64,
    pub best_move: Option<Move>,
    pub depth: u32,
    pub node: NodeKind,
}

impl TableEntry {
    pub fn is_all(&self) -> bool {
        matches!(self.node, NodeKind::All(_))
    }
}

static TABLE: SyncLazy<Table> = SyncLazy::new(Table::new);

pub fn initialize() {
    SyncLazy::force(&TABLE);
}

pub fn clear() {
    TABLE.clear()
}

pub fn query(pos: &Position) -> Option<Entry<'_>> {
    TABLE.query(pos)
}

pub fn record_pv(pos: &Position, best_move: Move, depth: u32, value: Value) {
    TABLE.record_pv(pos, best_move, depth, value);
}

pub fn record_cut(pos: &Position, best_move: Move, depth: u32, value: Value) {
    TABLE.record_cut(pos, best_move, depth, value);
}

pub fn record_all(pos: &Position, depth: u32, value: Value) {
    TABLE.record_all(pos, depth, value);
}

/// Looks up the principal variation from the given position to the given depth. This is the line that the engine
/// is pursuing.
pub fn get_pv(pos: &Position, depth: u32) -> Vec<Move> {
    let mut pv = vec![];
    let mut pv_clone = pos.clone();
    for _ in 0..depth {
        if let Some(best_move) = query(pos).and_then(|e| e.best_move()) {
            pv.push(best_move);
            pv_clone.make_move(best_move);
        } else {
            break;
        }
    }

    pv
}
