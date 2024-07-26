.code64
.global _long_mode_start

.section .text

_long_mode_start:
    # load 0 into all data segment registers
    mov ax, 0
    mov ss, ax
    mov dx, ax
    mov es, ax
    mov fs, ax
    mov gs, ax

    # call kernel main
    # extern kernel_main
    # call kernel_main

    # print `OK` to the VGA buffer
    mov eax, 0x0f4b0f4f
    mov dword ptr [0xb8000], eax
    hlt
