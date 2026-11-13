ENTRY(_start)

/* QEMU virt RAM starts at 0x40000000; leave its DTB below the image. */
MEMORY {
    RAM (rwx) : ORIGIN = 0x40080000, LENGTH = 0x07f80000
}

SECTIONS {
    .text : ALIGN(16) {
        KEEP(*(.text.boot))
        *(.text .text.*)
    } > RAM
    .rodata : ALIGN(16) { *(.rodata .rodata.*) } > RAM
    .data : ALIGN(16) { *(.data .data.*) *(.got .got.*) } > RAM
    .bss (NOLOAD) : ALIGN(16) {
        __bss_start = .;
        *(.bss .bss.*) *(COMMON)
        . = ALIGN(16);
        __bss_end = .;
    } > RAM
    .stack (NOLOAD) : ALIGN(16) {
        __stack_bottom = .;
        . += 1M;
        __stack_top = .;
        KEEP(*(.stack.secondary))
        __stacks_end = .;
    } > RAM
    /DISCARD/ : { *(.eh_frame .eh_frame_hdr) *(.comment) }
}

ASSERT((__stack_top & 15) == 0, "AAPCS64 requires 16-byte stack alignment")
ASSERT(__stacks_end <= ORIGIN(RAM) + LENGTH(RAM), "firmware exceeds guest RAM")
