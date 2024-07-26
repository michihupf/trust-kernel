#![no_std]
#![no_main]

// this makes sure we use the panic_handler from trust.
#[allow(unused_imports, clippy::single_component_path_imports)]
use trust;

core::arch::global_asm!(include_str!("arch/x86_64/boot.s"));
core::arch::global_asm!(include_str!("arch/x86_64/multiboot_header.s"));
core::arch::global_asm!(include_str!("arch/x86_64/long_mode_init.s"));
