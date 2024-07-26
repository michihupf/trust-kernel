#![no_std]
#![no_main]

use core::panic::PanicInfo;

use trust;

core::arch::global_asm!(include_str!("arch/x86_64/boot.s"));
core::arch::global_asm!(include_str!("arch/x86_64/multiboot_header.s"));
core::arch::global_asm!(include_str!("arch/x86_64/long_mode_init.s"));
