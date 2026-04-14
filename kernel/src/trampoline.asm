[org 0x8000]
[bits 16]

trampoline_start:
    cli
    cld

    ; 1. Load 32-bit GDT and enter Protected Mode
    lgdt [gdt32_ptr]
    mov eax, cr0
    or eax, 1
    mov cr0, eax

    jmp 0x08:protected_mode

[bits 32]
protected_mode:
    ; 2. Initialize segment registers
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov ss, ax

    ; 3. Setup Long Mode (PAE, LME, Paging)
    ; Assume P4 table is passed at a fixed physical address (e.g., 0x1000)
    mov eax, 0x1000
    mov cr3, eax

    mov eax, cr4
    or eax, 1 << 5 ; PAE
    mov cr4, eax

    mov ecx, 0xC0000080 ; EFER MSR
    rdmsr
    or eax, 1 << 8 ; LME
    wrmsr

    mov eax, cr0
    or eax, 1 << 31 ; PG
    mov cr0, eax

    lgdt [gdt64_ptr]
    jmp 0x18:long_mode

[bits 64]
long_mode:
    ; 4. Call kernel entry point for APs
    ; The entry point address should be placed at a fixed location by the BSP
    mov rax, [0x500]
    jmp rax

; GDTs and Pointers
gdt32:
    dq 0 ; null
    dq 0x00cf9a000000ffff ; code
    dq 0x00cf92000000ffff ; data
gdt32_ptr:
    dw $ - gdt32 - 1
    dd gdt32

gdt64:
    dq 0 ; null
    dq 0 ; null
    dq 0 ; null
    dq 0x00af9a000000ffff ; code
gdt64_ptr:
    dw $ - gdt64 - 1
    dq gdt64
