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

# ATTENTION! Kernel return is undefined behaviour and should never happen as enforced by
# the Rust langauge. We will trap execution anyway should the above call ever return for 
# some arbitrary weird reason:

.die:
    hlt
    jmp .die
