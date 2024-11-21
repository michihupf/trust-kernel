.intel_syntax noprefix
global long_mode_start

section .text
bits    64

long_mode_start:
	;   load 0 into all data segment registers
	mov ax, 0
	mov ss, ax
	mov dx, ax
	mov es, ax
	mov fs, ax
	mov gs, ax

	; call kernel_entrypoint
	extern kernel_entrypoint
	call kernel_entrypoint
	
	;   print `OK` to screen
	mov eax, 0x0f4b0f4f
	mov dword [0xb8000], eax
	hlt
