# Copyright 2021 Sean Gillespie.
#
# Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
# http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
# <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
# option. This file may not be copied, modified, or distributed
# except according to those terms.

from typing import Mapping, NamedTuple

from chess import Board, Move


class STSTest(NamedTuple):
    """
    A single test, derived from the Strategic Test Suite (STS).
    """

    fen: str
    best_move: Move
    id: str
    score_map: Mapping[Move, int]

    @staticmethod
    def from_epd(epd: str) -> "STSTest":
        board, meta = Board.from_epd(epd)
        fen = board.fen()
        best_move = meta["bm"][0]
        id = meta["id"]
        score_map = {}
        if "c0" in meta:
            for move in meta["c0"].split(" "):
                if move.endswith(","):
                    move = move[:-1]
                move_str, score = move.split("=")
                score_map[board.parse_san(move_str)] = int(score)
        else:
            score_map = {best_move: 10}
        return STSTest(fen=fen, best_move=best_move, id=id, score_map=score_map)
