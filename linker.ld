ENTRY(_start)
SECTIONS {
    . = 0xffffffff80000000;
    .text : { *(.text .text.*) }
    .rodata : { *(.rodata .rodata.*) }
    .data : {
        *(.data .data.*)
        . = ALIGN(8);
        KEEP(*(.limine_reqs))
    }
    .bss : { *(.bss .bss.*) }
}
