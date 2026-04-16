ENTRY(_start)

SECTIONS
{
    /* Load the kernel in the higher half */
    . = 0xffffffff80000000;

    .text : ALIGN(4096)
    {
        *(.text .text.*)
    }

    .rodata : ALIGN(4096)
    {
        *(.rodata .rodata.*)
    }

    .data : ALIGN(4096)
    {
        *(.data .data.*)
    }

    /* Limine requests section (8-byte alignment mandatory) */
    .requests : ALIGN(8)
    {
        KEEP(*(.requests_start))
        KEEP(*(.requests))
        KEEP(*(.requests_end))
    }

    .bss : ALIGN(4096)
    {
        *(.bss .bss.*)
        *(COMMON)
    }

    /* Sex-specific SASOS sections */
    .pdx_ring : ALIGN(4096)
    {
        *(.pdx_ring)
    }

    .gemini_repair : ALIGN(4096)
    {
        *(.gemini_repair)
    }

    /DISCARD/ : { *(.eh_frame) *(.comment) }
}
