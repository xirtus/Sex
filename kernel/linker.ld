ENTRY(_start)

SECTIONS
{
    . = 0x1000000;  /* PHYSICAL high-memory anchor – Limine will load here */

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

    .bss : ALIGN(4096)
    {
        *(.bss .bss.*)
        *(COMMON)
    }

    /* Sex-specific sections */
    .pdx_ring : ALIGN(4096)
    {
        *(.pdx_ring)
    }

    .gemini_repair : ALIGN(4096)
    {
        *(.gemini_repair)
    }

    /DISCARD/ : { *(.eh_frame) *(.comment) }
}o