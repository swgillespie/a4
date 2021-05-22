import json
import os
import subprocess
import sys


def collect_tests(dir):
    tests = []
    for (path, _, files) in os.walk(dir):
        for file in files:
            _, ext = os.path.splitext(file)
            if ext != ".json":
                continue
            with open(os.path.join(path, file)) as f:
                test = json.load(f)
                if test["kind"] == "perft":
                    for key, value in test["counts"].items():
                        tests.append(
                            {
                                "kind": "perft",
                                "fen": test["fen"],
                                "depth": int(key),
                                "count": value,
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


def main():
    tests = collect_tests(os.path.join(os.getcwd(), "tests"))
    passes = []
    fails = []
    for test in tests:
        result = run_perft_test(test)
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
        assert fail["test"]["kind"] == "perft"
        print(
            "  perft: {} (depth {}) => {} (expected {})".format(
                fail["test"]["fen"],
                fail["test"]["depth"],
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