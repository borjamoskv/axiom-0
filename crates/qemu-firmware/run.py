#!/usr/bin/env python3
"""Build and run the actual no_std ELF; check guest output and process status."""

import argparse
import math
from pathlib import Path
import re
import shutil
import subprocess
import sys


PACKAGE = Path(__file__).resolve().parent
TARGET = "aarch64-unknown-none"
BOOT = "AXIOM-0 BOOT: aarch64 EL1, no_std + alloc"
CHECKS = [
    "allocator alignment, exhaustion and growth",
    "QTT polymorphism and NbE",
    "QTT linearity and branch parity",
    "zero-quantity argument erasure",
    "natural recursion and C99 generation",
    "identity proof erasure",
    "atomic snapshots: 64 publications",
    "Turbine telemetry and rejected holes",
]
SMP_CHECK = "SMP snapshot contention"
SMP_METRICS = re.compile(
    r"SMP SNAPSHOTS: writes=32768 coherent=([0-9]{1,20}) "
    r"exhausted=([0-9]{1,20}) final_version=65536"
)


def build(mode: str) -> Path:
    artifacts = PACKAGE / "target" / mode
    command = [
        "cargo", "build", "--locked", "--release", "--target", TARGET,
        "--manifest-path", str(PACKAGE / "Cargo.toml"),
        "--target-dir", str(artifacts),
    ]
    if mode != "normal":
        command += ["--features", f"{mode}-test"]
    subprocess.run(command, cwd=PACKAGE, check=True)
    return artifacts / TARGET / "release" / "axiom0-qemu-firmware"


def execute(qemu: str, elf: Path, timeout: float, mode: str) -> None:
    smp = mode == "smp"
    log = PACKAGE / "target" / f"{mode}-qemu.log"
    command = [
        qemu, "-machine", "virt,secure=off,virtualization=off",
        "-cpu", "cortex-a53", "-accel", f"tcg,thread={'multi' if smp else 'single'}",
        "-smp", "2,sockets=1,clusters=1,cores=2,threads=1" if smp else "1",
        "-m", "128M", "-nodefaults",
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
        output = (error.stdout or b"").decode(errors="replace")
        log.write_text(f"exit_status=timeout\n{output}", encoding="utf-8")
        print(output, end="", flush=True)
        raise RuntimeError(f"QEMU did not terminate within {timeout:g}s") from error
    print(result.stdout, end="", flush=True)
    log.write_text(
        f"exit_status={result.returncode}\n{result.stdout}", encoding="utf-8"
    )
    lines = result.stdout.splitlines()
    checks = CHECKS + ([SMP_CHECK] if smp else [])
    expected = [f"[OK] {name}" for name in checks]
    observed = [line for line in lines if line.startswith("[OK] ")]
    if lines.count(BOOT) != 1 or observed != expected or any(line.startswith("[FAIL]") for line in lines):
        raise RuntimeError("firmware did not report all expected guest checks")
    success = f"FIRMWARE PASS: {len(checks)} checks"
    summaries = [line for line in lines if line.startswith("FIRMWARE PASS:")]
    panics = [line for line in lines if line.startswith("[PANIC]")]
    if mode == "panic":
        if result.returncode != 1 or len(panics) != 1 or summaries:
            raise RuntimeError("panic control did not report a guest panic with exit status 1")
        if "intentional panic self-test" not in lines:
            raise RuntimeError("panic control failed for an unexpected reason")
        print("HOST PASS: expected guest panic returned exit status 1", flush=True)
    else:
        if result.returncode != 0 or summaries != [success] or panics:
            raise RuntimeError(f"firmware failed (QEMU exit status {result.returncode})")
        if smp:
            metrics = [line for line in lines if line.startswith("SMP SNAPSHOTS:")]
            match = SMP_METRICS.fullmatch(metrics[0]) if len(metrics) == 1 else None
            if match is None or int(match[1]) < 2 or int(match[2]) == 0:
                raise RuntimeError("SMP output lacks coherent snapshots and observed contention")
        print("HOST PASS: guest completed all checks and returned exit status 0", flush=True)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--qemu", default="qemu-system-aarch64", help="QEMU executable")
    parser.add_argument("--timeout", type=float, default=30, help="guest timeout in seconds")
    parser.add_argument("--check-panic", action="store_true", help="also verify the panic handler")
    parser.add_argument("--smp", action="store_true", help="run the two-vCPU snapshot check")
    args = parser.parse_args()
    if not math.isfinite(args.timeout) or args.timeout <= 0:
        parser.error("--timeout must be finite and positive")
    qemu = shutil.which(args.qemu)
    if qemu is None:
        parser.error("qemu-system-aarch64 is missing; install QEMU or pass --qemu PATH")
    if shutil.which("cargo") is None:
        parser.error("cargo is missing; install the Rust toolchain")
    subprocess.run([qemu, "--version"], check=True)
    mode = "smp" if args.smp else "normal"
    execute(qemu, build(mode), args.timeout, mode)
    if args.check_panic:
        execute(qemu, build("panic"), args.timeout, "panic")
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (RuntimeError, subprocess.CalledProcessError, OSError) as error:
        print(f"HOST FAIL: {error}", file=sys.stderr)
        sys.exit(1)
