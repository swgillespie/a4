# Copyright 2021 Sean Gillespie.
#
# Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
# http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
# <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
# option. This file may not be copied, modified, or distributed
# except according to those terms.

"""
a4's test harness, for external testing.
"""

import asyncio
import json
import os
import subprocess
import sys

from chess import Board, Move
from chess.engine import Limit, INFO_ALL

from a4.uci import popen_release


def collect_tests(dir):
    tests = []
    for (path, _, files) in os.walk(dir):
        for file in files:
            _, ext = os.path.splitext(file)
            if ext != ".json":
                continue
            file_tests = load_tests_from_file(os.path.join(path, file))
            tests.extend(file_tests)
    return tests


def load_tests_from_file(path):
    tests = []
    with open(path) as f:
        test = json.load(f)
        if test["kind"] == "perft":
            for key, value in test["counts"].items():
                tests.append(
                    {
                        "path": path,
                        "kind": "perft",
                        "fen": test["fen"],
                        "depth": int(key),
                        "count": value,
                    }
                )
        elif test["kind"] == "quality":
            tests.append(
                {
                    "path": path,
                    "kind": "quality",
                    "positions": test["positions"],
                }
            )
    return tests


def run_perft_test(test):
    assert test["kind"] == "perft"
    count = int(
        subprocess.check_output(
            [
                "./target/release/a4-perft",
                test["fen"],
                "--depth",
                str(test["depth"]),
            ]
        )
        .decode("utf-8")
        .strip()
    )
    return {
        "test": test,
        "pass": test["count"] == count,
        "expected": test["count"],
        "actual": count,
    }


async def run_quality_test(test):
    assert test["kind"] == "quality"
    for i, pos in enumerate(test["positions"]):
        engine = await popen_release()
        try:
            bestmove = Move.from_uci(pos["bestmove"])
            board = Board(pos["fen"])
            info = await engine.play(board, Limit(time=0.5), info=INFO_ALL)
            if info.move != bestmove:
                return {
                    "test": test,
                    "index": i,
                    "pass": False,
                    "expected": bestmove,
                    "actual": info.move,
                }

        finally:
            await engine.quit()
    return {
        "test": test,
        "pass": True,
    }


def main():
    if len(sys.argv) == 2:
        tests = load_tests_from_file(sys.argv[1])
    else:
        tests = collect_tests(os.path.join(os.getcwd(), "tests"))
    passes = []
    fails = []
    for test in tests:
        if test["kind"] == "perft":
            result = run_perft_test(test)
        elif test["kind"] == "quality":
            result = asyncio.run(run_quality_test(test))
        if result["pass"]:
            print(".", end="")
            passes.append(result)
        else:
            print("!", end="")
            fails.append(result)
        sys.stdout.flush()
    print("\n")
    if fails:
        print("=====================================")
        print("Failed Tests:")
    for fail in fails:
        if fail["test"]["kind"] == "perft":
            print(
                "  ({}) perft: {} (depth {}) => {} (expected {})".format(
                    fail["test"]["path"],
                    fail["test"]["fen"],
                    fail["test"]["depth"],
                    fail["actual"],
                    fail["expected"],
                )
            )
        elif fail["test"]["kind"] == "quality":
            failed_pos = fail["test"]["positions"][fail["index"]]
            print(
                "  ({}) quality: {} => {} (expected {})".format(
                    fail["test"]["path"],
                    failed_pos["fen"],
                    fail["actual"],
                    fail["expected"],
                )
            )
    if fails:
        print("=====================================")
    print(f"{len(passes)}/{len(tests)} passed")
    sys.exit(0 if len(passes) == len(tests) else 1)


if __name__ == "__main__":
    main()
