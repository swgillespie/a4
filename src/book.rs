// Copyright 2017-2022 Sean Gillespie.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::lazy::SyncLazy;

use rand::prelude::SliceRandom;
use serde::Deserialize;

const BOOK_STR: &str = include_str!("book.json");

static BOOK: SyncLazy<PositionNode> =
    SyncLazy::new(|| serde_json::from_str(BOOK_STR).expect("failed to deserialize book"));

#[derive(Deserialize)]
struct PositionNode {
    #[serde(rename = "total")]
    _total: usize,
    moves: Vec<MoveNode>,
}

#[derive(Deserialize)]
struct MoveNode {
    #[serde(rename = "count")]
    _count: usize,
    #[serde(rename = "move")]
    mov: String,
    probability: f64,
    children: Option<PositionNode>,
}

pub fn query(sequence: &[String]) -> Option<String> {
    fn find_book_move<'a>(candidate: &str, book: &'a [MoveNode]) -> Option<&'a MoveNode> {
        for book_move in book {
            if candidate == book_move.mov {
                return Some(book_move);
            }
        }

        return None;
    }

    let mut cursor: &PositionNode = &*BOOK;
    for mov in sequence {
        if let Some(book_move) = find_book_move(mov, &cursor.moves) {
            if let Some(child) = &book_move.children {
                cursor = child;
            } else {
                return None;
            }
        }
    }

    let candidates: Vec<_> = cursor
        .moves
        .iter()
        .map(|node| (node.mov.clone(), node.probability))
        .collect();
    let (mov, _) = candidates
        .choose_weighted(&mut rand::thread_rng(), |i| i.1)
        .expect("failed to sample RNG");
    Some(mov.clone())
}
