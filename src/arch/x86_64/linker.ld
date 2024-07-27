ENTRY(start)

PAGE_SIZE = 4K;

SECTIONS {
  /* set load address to 1 MiB */
  . = 1M;

  .rodata :
  {
    /* ensure that the multiboot header is at the beginning */
    KEEP(*(.multiboot_header))
    *(.rodata .rodata.*)
    . = ALIGN(PAGE_SIZE);
  }

  .text :
  {
    *(.text .text.*)
    . = ALIGN(PAGE_SIZE);
  }

  .data :
  {
    *(.data .data.*)
    . = ALIGN(PAGE_SIZE);
  }

  .bss :
  {
    *(.bss .bss.*)
    . = ALIGN(PAGE_SIZE);
  }

  .got :
  {
    *(.got)
    *(.igot)
    . = ALIGN(PAGE_SIZE);
  }

  .got.plt : ALIGN(PAGE_SIZE) {
    *(.got.plt)
    *(.igot.plt)
  }

  .data.rel.ro : ALIGN(PAGE_SIZE) {
    *(.data.rel.ro.local*) *(.data.rel.ro .data.rel.ro.*)
  }

  .gcc_except_table : ALIGN(PAGE_SIZE) {
    *(.gcc_except_table)
  }

  /DISCARD/ : {
    *(.group)
    *(.note.*)
    *(.comment)
    *(.interp)
    *(.init)
    *(.fini)
  }
}
