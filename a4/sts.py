# Copyright 2021 Sean Gillespie.
#
# Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
# http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
# <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
# option. This file may not be copied, modified, or distributed
# except according to those terms.

import argparse
import asyncio
from typing import List, Mapping, NamedTuple
from pathlib import Path

from chess import Board, Move
from chess.engine import Limit, UciProtocol

from a4.uci import popen_release

parser = argparse.ArgumentParser(
    description="Power estimation for UCI chess engines via the STS."
)
parser.add_argument(
    "-j", metavar="JOBS", help="Number of concurrent jobs to launch", default=1
)
parser.add_argument(
    "--github",
    action="store_true",
    help="Run additional GitHub Actions magic",
    default=False,
)


class TestResult(NamedTuple):
    score: int
    move: Move


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

    async def execute(self, protocol: UciProtocol) -> TestResult:
        board = Board(fen=self.fen)
        result = await protocol.play(board, Limit(time=1))
        score = self.score_map.get(result.move) or 0
        return TestResult(score=score, move=result.move)


def collect_tests_from_file(file: Path) -> List[STSTest]:
    with open(file) as file:
        return [STSTest.from_epd(line) for line in file]


async def run_single(sem: asyncio.BoundedSemaphore, test: STSTest):
    async with sem:
        engine = await popen_release()
        try:
            return await test.execute(engine)
        finally:
            await engine.quit()
            print(".", end="", flush=True)


async def run():
    args = parser.parse_args()
    tests = collect_tests_from_file("tests/sts/STS1.epd")
    sem = asyncio.BoundedSemaphore(int(args.j))
    futures = [run_single(sem, test) for test in tests]
    results = await asyncio.gather(*futures)
    total_score = sum(map(lambda x: x.score, results))
    print(f"\nTotal Score = {total_score}")
    if args.github:
        print(f"::set-output name=Score::{total_score}")


def main():
    asyncio.run(run())


if __name__ == "__main__":
    main()
