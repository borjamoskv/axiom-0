#!/usr/bin/env python3
"""Validate the hosted kernel, no_std library and real QEMU firmware together."""

import argparse
import math
import os
from pathlib import Path
import shlex
import shutil
import subprocess
import sys


ROOT = Path(__file__).resolve().parent
FIRMWARE = "crates/qemu-firmware/Cargo.toml"
TARGET = "aarch64-unknown-none"


def validation_steps(qemu: str, timeout: float) -> list[tuple[str, list[str]]]:
    guest = [
        sys.executable, "crates/qemu-firmware/run.py",
        "--qemu", qemu, "--timeout", str(timeout),
    ]
    firmware_clippy = [
        "cargo", "clippy", "--locked", "--manifest-path", FIRMWARE,
        "--target", TARGET,
    ]
    return [
        ("Validation runner regressions", [
            sys.executable, "-B", "-m", "unittest", "discover",
            "-s", "scripts", "-p", "test_*.py", "-v",
        ]),
        ("Core formatting", ["cargo", "fmt", "--all", "--", "--check"]),
        ("Firmware formatting", [
            "cargo", "fmt", "--manifest-path", FIRMWARE, "--", "--check",
        ]),
        # Keep the three AGENTS.md commands verbatim.
        ("AGENTS core check", ["cargo", "check", "--workspace", "-q"]),
        ("AGENTS core tests", ["cargo", "test", "--workspace", "-q"]),
        ("AGENTS core Clippy", [
            "cargo", "clippy", "--workspace", "--", "-D", "warnings",
        ]),
        # Preserve the previous CI coverage of examples and all hosted targets.
        ("Hosted all-target Clippy", [
            "cargo", "clippy", "--locked", "--all-targets", "--", "-D", "warnings",
        ]),
        ("Hosted all-target tests", ["cargo", "test", "--locked", "--all-targets", "-q"]),
        ("no_std host tests", [
            "cargo", "test", "--locked", "--workspace", "--no-default-features", "-q",
        ]),
        ("no_std host Clippy", [
            "cargo", "clippy", "--locked", "--workspace", "--no-default-features",
            "--", "-D", "warnings",
        ]),
        ("Bare-metal library", [
            "cargo", "build", "--locked", "--lib", "--release",
            "--no-default-features", "--target", TARGET,
        ]),
        ("Firmware default Clippy", firmware_clippy + ["--", "-D", "warnings"]),
        ("Firmware all-feature Clippy", firmware_clippy + [
            "--all-features", "--", "-D", "warnings",
        ]),
        ("Hosted 10k stress", [
            "cargo", "run", "--locked", "--release", "--example", "stress_popperian_10k",
        ]),
        ("QEMU single-vCPU firmware", guest),
        ("QEMU SMP and panic control", guest + ["--smp", "--check-panic"]),
    ]


def run_steps(steps: list[tuple[str, list[str]]]) -> int:
    for index, (name, command) in enumerate(steps, start=1):
        print(f"\n[{index}/{len(steps)}] {name}\n$ {shlex.join(command)}", flush=True)
        try:
            subprocess.run(command, cwd=ROOT, check=True)
        except (subprocess.CalledProcessError, OSError) as error:
            print(f"VALIDATION FAIL [{name}] {error}", file=sys.stderr, flush=True)
            return 1
    print(f"\nVALIDATION PASS ({len(steps)} stages)", flush=True)
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--qemu", default="qemu-system-aarch64", help="QEMU executable")
    parser.add_argument("--timeout", type=float, default=30, help="timeout per QEMU guest in seconds")
    args = parser.parse_args(argv)
    if not math.isfinite(args.timeout) or args.timeout <= 0:
        parser.error("--timeout must be finite and positive")
    qemu = shutil.which(args.qemu)
    if qemu is None:
        parser.error("qemu-system-aarch64 is missing; install QEMU or pass --qemu PATH")
    for binary in ("cargo", os.environ.get("CC", "cc")):
        if shutil.which(binary) is None:
            parser.error(f"required executable is missing: {binary}")
    # Resolve an explicitly relative QEMU path before commands change directory.
    return run_steps(validation_steps(str(Path(qemu).resolve()), args.timeout))


if __name__ == "__main__":
    try:
        sys.exit(main())
    except KeyboardInterrupt:
        print("\nVALIDATION INTERRUPTED", file=sys.stderr)
        sys.exit(130)
