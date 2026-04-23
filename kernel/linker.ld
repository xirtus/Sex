ENTRY(_start)

/* Define the Higher-Half Base Address */
KERNEL_LMA = 0xffffffff80000000;

SECTIONS {
    /* The kernel starts at the 2MB physical mark, but mapped virtually to the higher half */
    . = KERNEL_LMA + 0x200000;

    .text : ALIGN(4K) {
        *(.text .text.*)
    }

    .rodata : ALIGN(4K) {
        *(.rodata .rodata.*)
    }

    .data : ALIGN(4K) {
        /* Limine 7.x request table must be in one RW segment — no page-permission splits */
        KEEP(*(.limine_requests_start))
        KEEP(*(.limine_requests))
        KEEP(*(.limine_requests_end))
        *(.data .data.*)
    }

    .bss : ALIGN(4K) {
        *(.bss .bss.*)
        *(COMMON)
    }
}
