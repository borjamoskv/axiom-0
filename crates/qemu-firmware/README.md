# AArch64 QEMU firmware smoke test

This standalone `aarch64-unknown-none` executable boots the real AXIOM-0 library
with `default-features = false`. It provides its own entry point, stack, heap,
global allocator and panic handler. There is no guest `std`, operating system,
libc or external Rust dependency. Python, Cargo and the small `build.rs` run on
the host, not inside the guest.

## Validate the core and firmware together

From the repository root, run the shared local and CI entrypoint.

```sh
python3 validate.py
```

This runs the three AGENTS.md controls, hosted formatting/all-target/stress
checks, `no_std` host tests and Clippy, the bare-metal release library build,
firmware formatting and Clippy with default and all features, then the normal,
SMP and separate panic QEMU images. The first failure stops the suite.
Every stage is required, including QEMU. See the [root quickstart](../../README.md)
for Rust components, target, Python 3.10 or newer, native C99 compiler and emulator prerequisites.

The validator resolves the repository from its own path, so an absolute path to
`validate.py` works from any directory. `--qemu` selects the emulator and
`--timeout` sets the finite positive timeout per guest run, defaulting to 30
seconds. Both options are passed to the firmware runner. The commands below
remain available for focused firmware checks.

## Reproduce from the repository root

Requirements: a stable Rust toolchain supporting edition 2024, Python 3 and
`qemu-system-aarch64` on `PATH`. The validated run used Rust 1.97.1 and QEMU
11.1.2 on macOS/Apple Silicon with TCG emulation.

```sh
rustup target add aarch64-unknown-none
# macOS, if QEMU is missing:
brew install qemu

python3 crates/qemu-firmware/run.py --check-panic
```

The runner builds a release ELF, runs the eight guest checks, and requires both
the success marker and QEMU exit status **0**. With `--check-panic`, it builds a
second image that executes the same checks and deliberately panics. That control
must print `[PANIC]` and exit with status **1**, without the overall success
marker. Only that expected combination counts as a successful panic test.
The Python runner itself returns **0** when both runs meet those expectations.

`--qemu /absolute/path/qemu-system-aarch64` selects another binary. `--timeout 30`
sets a host-side timeout per guest run; timeout, missing markers, guest failures
and unexpected process status all fail the runner. The timeout must be finite
and positive; NaN, infinities, zero and negative values are rejected before
building or launching QEMU. The timeout is a test harness watchdog, not a latency
guarantee for the kernel.

The normal output is:

```text
AXIOM-0 BOOT: aarch64 EL1, no_std + alloc
[OK] allocator alignment, exhaustion and growth
[OK] QTT polymorphism and NbE
[OK] QTT linearity and branch parity
[OK] zero-quantity argument erasure
[OK] natural recursion and C99 generation
[OK] identity proof erasure
[OK] atomic snapshots: 64 publications
[OK] Turbine telemetry and rejected holes
Heap used: 119915 / 4194304 bytes (bump, no reclamation)
FIRMWARE PASS: 8 checks
HOST PASS: guest completed all checks and returned exit status 0
```

Heap usage can change with compiler versions or test changes. The additional
panic run ends with `intentional panic self-test` and
`HOST PASS: expected guest panic returned exit status 1`.

The runner saves guest output and exit status to `target/normal-qemu.log` and
`target/panic-qemu.log` inside this package. The normal and panic ELF images are
stored separately under `target/normal/` and `target/panic/`.

## Exercise snapshot contention on two vCPUs

```sh
python3 crates/qemu-firmware/run.py --smp --check-panic
```

`--smp` builds with `smp-test` and boots two Cortex-A53 vCPUs using
`-accel tcg,thread=multi`. CPU0 starts CPU1 with PSCI `CPU_ON64` over HVC.
CPU1 enters Rust on its own 64 KiB stack. Both CPUs install the same
identity mapping of guest RAM as Normal Inner Shareable memory before they
access the shared cell. [QEMU PSCI boot setup](https://github.com/qemu/qemu/blob/v11.1.2/hw/arm/boot.c#L1148-L1174) ·
[Multi-threaded TCG](https://www.qemu.org/docs/master/devel/multi-thread-tcg.html)

CPU1 publishes 32,768 versions of a 128-word cell. It waits after the first
publication for CPU0 to verify the checkpoint. CPU0 checks that each accepted
snapshot has an even, nondecreasing version and that all words match its
generation. The final version must be 65,536. CPU0 also counts
`RetryBudgetExhausted` from `try_read(1)` during the writer phase. If scheduling
produces zero exhaustions, the run fails with `no contention observed`.
The host timeout bounds the run, including the startup handshake.

The SMP run reports `FIRMWARE PASS: 9 checks` and one `SMP SNAPSHOTS` line with
the write count, accepted reads, exhausted reads and final version. Accepted and
exhausted counts vary by run. The ELF and log are under `target/smp/` and
`target/smp-qemu.log` respectively. A timed-out guest writes its partial output
and `exit_status=timeout` to the corresponding log. The optional panic control
boots its single-vCPU image after the SMP run.

To exercise a topology failure, use the manual QEMU command below with the
SMP ELF from `target/smp/` and keep `-smp 1`. The guest reports
`PSCI CPU_ON status: -2` and exits with status 1.

## Build and run manually

From the repository root:

```sh
cargo build --manifest-path crates/qemu-firmware/Cargo.toml \
  --target aarch64-unknown-none --release \
  --target-dir crates/qemu-firmware/target/normal

qemu-system-aarch64 \
  -machine virt,secure=off,virtualization=off \
  -cpu cortex-a53 -accel tcg,thread=single -smp 1 -m 128M \
  -nodefaults -display none -serial none -monitor none -no-reboot \
  -semihosting-config enable=on,target=native \
  -kernel crates/qemu-firmware/target/normal/aarch64-unknown-none/release/axiom0-qemu-firmware
```

The `.cargo/config.toml` also selects the target when building from this package
directory. `build.rs` passes an absolute linker-script path, so root-level
`--manifest-path` builds work too. Rust's bundled linker is sufficient.

This package has its own workspace because its platform-specific entry point
must not be compiled as a hosted root executable. The unified validator invokes
this workspace explicitly. Its Clippy checks can also be run individually.

```sh
cargo clippy --manifest-path crates/qemu-firmware/Cargo.toml \
  --locked --target aarch64-unknown-none -- -D warnings
cargo clippy --manifest-path crates/qemu-firmware/Cargo.toml \
  --locked --target aarch64-unknown-none --all-features -- -D warnings
```

The shared entrypoint includes the root validation contract as well.

```sh
cargo check --workspace -q
cargo test --workspace -q
cargo clippy --workspace -- -D warnings
cargo test --workspace --no-default-features -q
```

## Boot and allocator invariants

- The image is linked at `0x40080000` in QEMU `virt` RAM, above its DTB. QEMU loads
  the ELF segments. The runner configures EL1, secure/virtualization disabled
  and 128 MiB of RAM. An unexpected exception level or CPU affinity parks before
  entering Rust.
- `boot.S` masks interrupts, establishes a 16-byte-aligned 1 MiB stack, enables
  FP/SIMD for Rust's target ABI and zeroes BSS before calling `firmware_main`.
  The SMP image then enables the shared identity mapping. CPU1 sets up its own
  translation registers and stack, and uses the BSS already initialized by CPU0.
- The allocator owns a 4 MiB static backing allocation. Checked alignment/size
  arithmetic and CAS reserve disjoint ranges while preserving pointer provenance.
  Failed allocation returns null without advancing its cursor. Deallocation is a
  no-op: this finite, single-boot test does not reclaim memory or support a
  long-running allocation workload. The allocator test checks 4096-byte alignment,
  recoverable exhaustion and a successful allocation after exhaustion.
- Console formatting and the panic handler do not allocate. Arm A64 semihosting
  `SYS_WRITEC` emits characters and `SYS_EXIT` reports a process status through a
  two-word argument block. These operations require QEMU TCG semihosting enabled;
  they are the only host services used by the guest.
- Unsafe operations are restricted to this platform package: the allocator's
  storage contract, startup assembly, exported entry point and semihosting trap.
  Each carries its safety invariant. The AXIOM-0 library retains
  `#![forbid(unsafe_code)]`.

## Evidence and limits

This runs the compiled ARM ELF in QEMU, including heap allocation, parsing,
explicit polymorphism, NbE, linearity/branch-parity rejection, proof erasure,
natural recursion and atomic telemetry. Scalar C99 generation is exercised as
Rust string generation; the guest does not compile or execute the resulting C.

The polymorphism check also exercises an accepted implicit type parameter and
rejects three plicity mismatches: an implicit lambda against an explicit function
type, an explicit lambda against an implicit function type, and an implicit
argument supplied to an explicit function. These assertions remain within the
same eight guest check groups.

The recursion check rejects implicit motive and step parameters, since natural
induction applies those arguments explicitly. The identity check rejects a
non-function motive, a motive missing its proof parameter, and an annotated
motive with an incompatible endpoint domain. Valid recursion and identity
elimination still run in the same eight groups.

The single-vCPU run performs 64 complete snapshot publications, checks cache
alignment and rejects a zero read budget. The SMP run exercises an overlapping
reader and writer on two emulated vCPUs and requires observed contention.
These runs cover the tested QEMU configurations. Hosted tests and the finite
snapshot model provide complementary coverage of memory ordering.

Platform references: [QEMU virt boot contract](https://www.qemu.org/docs/master/system/arm/virt.html#hardware-configuration-information-for-bare-metal-programming),
[QEMU semihosting](https://www.qemu.org/docs/master/about/emulation.html#semihosting),
and [Arm's A64 semihosting ABI](https://github.com/ARM-software/abi-aa/blob/main/semihosting/semihosting.rst).
