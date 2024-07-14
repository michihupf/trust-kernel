//! # tRust
//!
//! A small kernel written in Rust to learn about how kernels work.

#![no_std]
#![cfg_attr(test, no_main)]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]
// enable x86-interrupt ABI
#![feature(abi_x86_interrupt)]
#![feature(asm_const)]
// needed for implementing a linked list allocator
#![feature(const_mut_refs)]

pub mod gdt;
pub mod heap;
pub mod idt;
pub mod memory;
pub mod serial;
pub mod task;
pub mod vga_buffer;

#[allow(unused_imports)]
use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;

extern crate alloc;

#[cfg(test)]
entry_point!(test_kernel_main);

/// Entry point for `cargo test`
#[cfg(test)]
fn test_kernel_main(_boot_info: &'static BootInfo) -> ! {
    // do initialization before testing

    init();
    test_main();

    // halt the CPU
    hlt_forever();
}

/// Initializes important systems like IDT, GDT and PIC8259.
pub fn init() {
    gdt::init();
    idt::init_idt();

    print!("Initializing 8259 PIC... ");
    unsafe { idt::PICS.lock().initialize() };
    println!("[ok]");

    x86_64::instructions::interrupts::enable();
    println!("Enabled external interrupts.");
}

pub fn hlt_forever() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Fail = 0x11,
}

/** Helper function to exit qemu with a provided exit code. Qemu will alter this
* exit code to final exit code of `(code << 1) | 1`. This should not be used for
* standard shutdown implementation and is only used for testing the kernel.
*
* Shutdown implemtation should follow the APM and/or ACPI power management standard.
*/
pub fn exit_qemu(exit_code: QemuExitCode) {
    use x86_64::instructions::port::Port;

    unsafe {
        let mut port = Port::new(0xf4);
        port.write(exit_code as u32);
    }
}

/// Generic trait to implement test debug logging
pub trait Testable {
    fn run(&self);
}

impl<T> Testable for T
where
    T: Fn(),
{
    /// Function to run a test and print success state.
    fn run(&self) {
        serial_print!("{}...\t", core::any::type_name::<T>());
        self();
        serial_println!("\r[ok] {}", core::any::type_name::<T>());
    }
}

/// Helper function that is called by the kernel entry point when in test config
/// to run tests.
pub fn test_runner(tests: &[&dyn Testable]) {
    serial_println!("Running {} tests...", tests.len());
    for test in tests {
        test.run();
    }

    exit_qemu(QemuExitCode::Success);
}

pub fn test_panic_handler(info: &PanicInfo) -> ! {
    serial_println!("[failed]\n");
    serial_println!("{}\n", info);
    exit_qemu(QemuExitCode::Fail);

    // CPU never halts because we exit qemu before
    hlt_forever();
}

/// This function is called on panic when in test mode and logs the error message
/// to the hosts stdout via a serial connection. Exits qemu after panic.
#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    serial_println!("[failed]\n");
    serial_println!("{}", info);
    exit_qemu(QemuExitCode::Fail);

    hlt_forever();
}
