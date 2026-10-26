#!/usr/bin/env python3
"""Build and run the actual no_std ELF; check guest output and process status."""

import argparse
from pathlib import Path
import shutil
import subprocess
import sys


PACKAGE = Path(__file__).resolve().parent
TARGET = "aarch64-unknown-none"
SUCCESS = "FIRMWARE PASS: 8 checks"
BOOT = "AXIOM-0 BOOT: aarch64 EL1, no_std + alloc"


def build(panic_test: bool) -> Path:
    artifacts = PACKAGE / "target" / ("panic" if panic_test else "normal")
    command = [
        "cargo", "build", "--release", "--target", TARGET,
        "--manifest-path", str(PACKAGE / "Cargo.toml"),
        "--target-dir", str(artifacts),
    ]
    if panic_test:
        command += ["--features", "panic-test"]
    subprocess.run(command, cwd=PACKAGE, check=True)
    return artifacts / TARGET / "release" / "axiom0-qemu-firmware"


def execute(qemu: str, elf: Path, timeout: float, panic_test: bool) -> None:
    command = [
        qemu, "-machine", "virt,secure=off,virtualization=off",
        "-cpu", "cortex-a53", "-accel", "tcg,thread=single",
        "-smp", "1", "-m", "128M", "-nodefaults",
        "-display", "none", "-serial", "none", "-monitor", "none",
        "-no-reboot", "-semihosting-config", "enable=on,target=native",
        "-kernel", str(elf),
    ]
    try:
        result = subprocess.run(
            command, cwd=PACKAGE, stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT, text=True, errors="replace", timeout=timeout,
        )
    except subprocess.TimeoutExpired as error:
        if error.stdout:
            print(error.stdout.decode(errors="replace"), end="", flush=True)
        raise RuntimeError(f"QEMU did not terminate within {timeout:g}s") from error
    print(result.stdout, end="", flush=True)
    label = "panic" if panic_test else "normal"
    (PACKAGE / "target" / f"{label}-qemu.log").write_text(
        f"exit_status={result.returncode}\n{result.stdout}", encoding="utf-8"
    )
    if BOOT not in result.stdout or result.stdout.count("[OK] ") != 8 or "[FAIL]" in result.stdout:
        raise RuntimeError("firmware did not execute all eight checks successfully")
    if panic_test:
        if result.returncode != 1 or "[PANIC]" not in result.stdout or SUCCESS in result.stdout:
            raise RuntimeError("panic control did not report a guest panic with exit status 1")
        if "intentional panic self-test" not in result.stdout:
            raise RuntimeError("panic control failed for an unexpected reason")
        print("HOST PASS: expected guest panic returned exit status 1", flush=True)
    else:
        if result.returncode != 0 or SUCCESS not in result.stdout or "[PANIC]" in result.stdout:
            raise RuntimeError(f"firmware failed (QEMU exit status {result.returncode})")
        print("HOST PASS: guest completed all checks and returned exit status 0", flush=True)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--qemu", default="qemu-system-aarch64", help="QEMU executable")
    parser.add_argument("--timeout", type=float, default=30, help="guest timeout in seconds")
    parser.add_argument("--check-panic", action="store_true", help="also verify the panic handler")
    args = parser.parse_args()
    if args.timeout <= 0:
        parser.error("--timeout must be positive")
    qemu = shutil.which(args.qemu)
    if qemu is None:
        parser.error("qemu-system-aarch64 is missing; install QEMU or pass --qemu PATH")
    if shutil.which("cargo") is None:
        parser.error("cargo is missing; install the Rust toolchain")
    subprocess.run([qemu, "--version"], check=True)
    execute(qemu, build(False), args.timeout, False)
    if args.check_panic:
        execute(qemu, build(True), args.timeout, True)
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (RuntimeError, subprocess.CalledProcessError, OSError) as error:
        print(f"HOST FAIL: {error}", file=sys.stderr)
        sys.exit(1)
