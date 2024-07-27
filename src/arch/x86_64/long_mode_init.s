.code64
.global _long_mode_start
.extern kernel_main

.section .text

_long_mode_start:
    # load 0 into all data segment registers
    mov ax, 0
    mov ss, ax
    mov dx, ax
    mov es, ax
    mov fs, ax
    mov gs, ax

    call kernel_main

    # kernel should never return from Rust at this point.
    # trap it if it happens anyway
.die:
    hlt
    jmp .die
