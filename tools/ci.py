import sys
import subprocess


def main() -> int:
    print("=> Checking format")
    subprocess.call(["cargo", "fmt", "--all", "--", "--check"])
    print("=> Building debug")
    subprocess.call(["cargo", "build", "--verbose"])
    print("=> Building release")
    subprocess.call(["cargo", "build", "--release", "--verbose"])
    print("=> Running internal tests")
    subprocess.call(["cargo", "test", "--verbose"])
    print("=> Running external tests")
    subprocess.call(["poetry", "run", "python3", "-m", "a4.tests"])
    print("=> Running pytest tests")
    subprocess.call(["poetry", "run", "pytest"])
    print("=> Done!")


if __name__ == "__main__":
    sys.exit(main())
