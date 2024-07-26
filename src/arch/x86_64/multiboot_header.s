.section .mutliboot_header

header_start:
    .int 0xE85250D6        # magic number (multiboot 2)
    .int 0                 # protected mode i386
    .int header_end - header_start  # header length
    # checksum
    .int 0x100000000 - (0xE85250D6 + 0 + (header_end - header_start))

    # optional multiboot tags here

    # required end tag
    .word 0  # type
    .word 0  # flags
    .int 8   # size
header_end:
