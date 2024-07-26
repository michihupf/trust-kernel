.code32
.global start
.extern _long_mode_start

.section .text

start:
	mov esp, stack_top
	mov edi, ebx # move multiboot info pointer to edi

	#    perform checks
	call check_multiboot
	call check_cpuid
	call check_long_mode

	#    setup paging
	call setup_page_tables # prepare page tables
	call enable_paging

	#    load 64-bit GDT
	lgdt [gdt64pointer]
	mov es, [gdt_offset]
	jmp es:_long_mode_start

check_multiboot:
	cmp eax, 0x36d76289
	jne .no_multiboot
	ret

.no_multiboot:
	mov al, '0'
	jmp error

check_cpuid:
	# Check if CPUID is supported by attempting to flip ID bit (bit 21)
	# in the FLAGS register. If flip succeeds, CPUID is available.

	#   Copy FLAGS to eax and ecx
	pushfd
	pop eax
	mov ecx, eax

	#   Flip ID
	xor eax, 0x00200000

	#    Apply FLAGS
	push eax
	popfd

	#   Copy FLAGS back
	pushfd
	pop eax

	#    Restore FLAGS to previous values
	push ecx
	popfd

	#   Check if flip occured
	cmp eax, ecx
	je  .no_cpuid
	ret

.no_cpuid:
	mov al, '1'
	jmp error

check_long_mode:
	#     test if extended processor info is available
	mov   eax, 0x80000000 # implicit argument for cpuid
	cpuid # get highest supported argument
	cmp   eax, 0x80000001 # if less CPU too old for long mode
	jb    .no_long_mode

	#     use extended info to test if long mode is supported
	mov   eax, 0x80000001 # argument for extended processor info
	cpuid # returns various feature bits in ecx and edx
	test  edx, 0x20000000 # test if LM-bit (long mode bit) is set
	jz    .no_long_mode
	ret

.no_long_mode:
	mov al, '2'
	jmp error

setup_page_tables:
	#   map last P4 entry to P4 table for recursive access later
	mov eax, p4_table
	or eax, 0b11 # present + writable
	mov dword ptr [p4_table + 511 * 8], eax
	
	#   map first P4 entry to P3 table
	mov eax, p3_table
	or  eax, 0b11 # present + writable
	mov dword ptr [p4_table], eax

	#   map first P3 entry to P2 table
	mov eax, p2_table
	or  eax, 0b11 # present + writable
	mov dword ptr [p3_table], eax

	#   map each P2 entry to a huge 2MiB page
	mov ecx, 0 # counter

.map_p2_table:
	#   map ecx-th P2 entry to a huge page that starts at address 2MiB*ecx
	mov eax, 0x200000 # 2MiB
	mul ecx
	or  eax, 0b10000011 # present + writable + huge
	mov dword ptr [p2_table + ecx * 8], eax # map ecx-th entry

	inc ecx
	cmp ecx, 512 # we have 512 table entries
	jne .map_p2_table

	ret

enable_paging:
	#   load P4 to CR3
	mov eax, p4_table
	mov cr3, eax

	#   enable PAE in cr4
	mov eax, cr4
	or  eax, 1 << 5
	mov cr4, eax

	#   set long mode bit in EFER MSR
	mov ecx, 0xc0000080 # points to EFER
	rdmsr
	or  eax, 1 << 8
	wrmsr

	#   enable
	mov eax, cr0
	or  eax, 1 << 31
	mov cr0, eax

	ret

error:
	# Prints `ERR` and the given error code to screen and hangs.
	# parameter: error code (in ascii) in al

	mov dword ptr [0xb8000], 0x0f520f45
	mov dword ptr [0xb8004], 0x0f3a0f52
	mov dword ptr [0xb8008], 0x0f200f20
	mov [0xb800a], al
	hlt

.section .rodata
gdt64:
	.quad 0 # 0-entry
gdt64code:
	.equ gdt_offset, . - gdt64
	.quad  (1<<43) | (1<<44) | (1<<47) | (1<<53) # code segment
gdt64pointer:
	.word . - gdt64 - 1
	.quad gdt64

.section .bss
.align   4096

.lcomm p4_table, 4096
.lcomm p3_table, 4096
.lcomm p2_table, 4096
.lcomm p1_table, 4096

.lcomm stack_bottom, 16384
stack_top:
