// Boot invariant: QEMU virt, secure=off, virtualization=off, one CPU, EL1,
// MMU disabled, and all ELF PT_LOAD segments initialized by QEMU.
.section .text.boot, "ax"
.global _start
.type _start, %function
_start:
    msr daifset, #0xf
    mrs x0, mpidr_el1
    and x0, x0, #0xff
    cbnz x0, .Lpark
    mrs x0, CurrentEL
    cmp x0, #4
    b.ne .Lpark
    adrp x0, __stack_top
    add x0, x0, :lo12:__stack_top
    mov sp, x0
    // Rust's target ABI permits FP/SIMD instructions; enable them before Rust.
    mrs x0, cpacr_el1
    orr x0, x0, #(3 << 20)
    msr cpacr_el1, x0
    isb
    adrp x0, __bss_start
    add x0, x0, :lo12:__bss_start
    adrp x1, __bss_end
    add x1, x1, :lo12:__bss_end
.Lzero:
    cmp x0, x1
    b.hs .Lrust
    str xzr, [x0], #8
    b .Lzero
.Lrust:
    bl firmware_main
.Lpark:
    wfe
    b .Lpark
.size _start, . - _start
