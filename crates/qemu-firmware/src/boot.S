// QEMU virt enters EL1 with translation disabled and loads the ELF segments.
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
    msr spsel, #1
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
.if {smp}
    // Complete the uncached BSS stores before either CPU uses the Normal map.
    dsb sy
    bl .Lenable_ram
.endif
    bl firmware_main
.Lpark:
    wfe
    b .Lpark
.size _start, . - _start

.if {smp}
.balign 4
.global secondary_start
.type secondary_start, %function
secondary_start:
    msr daifset, #0xf
    mrs x0, CurrentEL
    cmp x0, #4
    b.ne .Lpark
    mrs x0, mpidr_el1
    and x0, x0, #0xff
    cmp x0, #1
    b.ne .Lpark
    msr spsel, #1
    adrp x0, .Lsecondary_stack_top
    add x0, x0, :lo12:.Lsecondary_stack_top
    mov sp, x0
    mrs x0, cpacr_el1
    orr x0, x0, #(3 << 20)
    msr cpacr_el1, x0
    isb
    bl .Lenable_ram
    bl firmware_secondary
    b .Lpark
.size secondary_start, . - secondary_start

// Both EL1 CPUs use the same immutable, identity-mapped RAM block.
.Lenable_ram:
    mov x0, #0xff
    msr mair_el1, x0
    // 39-bit VAs, 4 KiB granule, WBWA/Inner Shareable walks, TTBR1 disabled.
    movz x0, #0x3519
    movk x0, #0x8099, lsl #16
    msr tcr_el1, x0
    adrp x0, .Lram_table
    add x0, x0, :lo12:.Lram_table
    msr ttbr0_el1, x0
    isb
    tlbi vmalle1
    dsb sy
    isb
    mrs x0, sctlr_el1
    orr x0, x0, #1
    orr x0, x0, #4
    orr x0, x0, #0x1000
    msr sctlr_el1, x0
    isb
    ret

.section .rodata.ram_table, "a"
.balign 4096
.Lram_table:
    .quad 0
    // L1 block at 0x40000000, AttrIndx 0, AF, Inner Shareable, EL1 RW.
    .quad 0x40000701
    .zero 4080

.section .stack.secondary, "aw", %nobits
.balign 16
    .skip 65536
.Lsecondary_stack_top:
.endif
