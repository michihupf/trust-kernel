//! # tRust
//!
//! A small kernel written in Rust to learn about how kernels work.

#![allow(internal_features)]
#![feature(
    const_mut_refs,
    lang_items,
    asm_const,
    // enable x86-interrupt ABI
    custom_test_frameworks,
    abi_x86_interrupt,
    ptr_internals,
    // rustc_private
)]
#![test_runner(test_runner)]
#![reexport_test_harness_main = "test_main"]
#![forbid(clippy::undocumented_unsafe_blocks)]
// #![warn(
//     clippy::all,
//     clippy::cargo,
//     clippy::complexity,
//     clippy::correctness,
//     clippy::pedantic,
//     clippy::perf,
//     clippy::style,
//     clippy::suspicious
// )]
// #![allow(clippy::cargo_common_metadata)]
#![no_std]
#![no_main]

pub mod acpi;
pub mod apic;
pub mod idt;
pub mod memory;
pub mod serial;
pub mod task;
pub mod vga_buffer;

use alloc::string::String;
#[allow(unused_imports)]
use core::panic::PanicInfo;
use core::{sync, time::Duration};
use multiboot2::{BootInformation, BootInformationHeader};
use task::{executor::Executor, keyboard, Task};
use x86_64::registers::control::{Cr0, Cr0Flags, Efer, EferFlags};

#[macro_use]
extern crate alloc;
extern crate rlibc;
// extern crate compiler_builtins;

#[macro_export]
macro_rules! status_print {
    ($msg:literal => $exp:expr) => {
        $crate::print!("{}... ", $msg);
        $exp;
        $crate::println!("[ok]");
    };
    ($msg:literal $($s:stmt);* $(;)*) => {
        $crate::print!("{}... ", $msg);
        $($s)*
        $crate::println!("[ok]");
    };
}

/// Constructs 64-bit bitmasks.
///
/// A helper macro to construct 64-bit bitmasks using either range-based syntax
/// or by passing specific bits that should be set to 1.
///
/// # Example
/// ```
/// // Set bits in a range to 1
/// const _: () = assert!(bitmask!(23..16) == 0x00ff_0000);
/// const _: () = assert!(bitmask!(0..0) == 0x0000_0000);
///
/// // Set specific bits to 1
/// const _: () = assert!(bitmask!(0, 2, 3) == 0b1101);
/// const _: () = assert!(bitmask!(18) == 1 << 18);
/// ```
#[macro_export]
macro_rules! bitmask {
    ($hi:literal..$lo:literal) => {{
        const _: () = assert!($hi >= $lo, "High bit was smaller than low bit.");
        ((1 << ($hi + 1)) - 1) ^ ((1 << $lo) - 1)
    }};
    ($($bit:literal),+) => {{
        $(1<<$bit)|*
    }};
}

/// Includes needed entry assembly.
///
/// Helper macro to include needed entry assembly. Make sure to only include it once!
#[macro_export]
macro_rules! entry_asm {
    () => {
        core::arch::global_asm!(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/arch/x86_64/multiboot_header.s"
        )));
        core::arch::global_asm!(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/arch/x86_64/boot.s"
        )));
        core::arch::global_asm!(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/arch/x86_64/long_mode_init.s"
        )));
    };
}

#[test_case]
fn bitmask_macro() {
    const _: () = assert!(bitmask!(23..16) == 0x00ff_0000);
    const _: () = assert!(bitmask!(0..0) == 0x0000_0001);
    const _: () = assert!(bitmask!(0, 2, 3) == 0b1101);
    const _: () = assert!(bitmask!(18) == 1 << 18);
}

#[cfg(test)]
entry_asm!();

#[cfg(test)]
#[no_mangle]
pub extern "C" fn kernel_entrypoint(mbi_ptr: usize) -> ! {
    kernel_main(mbi_ptr);
}

/// Kernel main!
pub fn kernel_main(mbi_ptr: usize) -> ! {
    // print "Booting" to the screen
    println!("Booting tRust...");

    // SAFETY: mbi is placed here by multiboot2 bootloader
    let mbi = unsafe { BootInformation::load(mbi_ptr as *const BootInformationHeader).unwrap() };

    let mut memory_controller = memory::init(&mbi);

    idt::init(&mut memory_controller);

    // SAFETY: this is not yet fully safe, but should not propose major issues // FIXME
    status_print!("initializing 8259 PIC" => unsafe { idt::PICS.lock().initialize() });

    // enable external interrupts
    status_print!("enabling external interrupts" => x86_64::instructions::interrupts::enable());

    // look for RSDP
    acpi::try_init(&mbi, &mut memory_controller);

    // // //  GENERAL  INIT  DONE  // // //
    // --   Tests may proceed below   -- //

    // run tests when in test config
    #[cfg(test)]
    {
        println!("Running tests");
        test_main();
    }

    print_cpu_info();

    #[cfg(test)]
    exit_qemu(QemuExitCode::Success);

    // test asynchronous tasks
    let mut executor = Executor::new();
    executor.spawn(Task::new(keyboard::print_keypresses()));
    executor.run();

    // hlt_forever();
}

#[lang = "eh_personality"]
#[no_mangle]
pub extern "C" fn eh_personality() {}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    test_panic_handler(info);
}

/// Enables the NO_EXECUTE bit in the Extended Feature Enable Register (EFER).
pub fn enable_nxe_bit() {
    // SAFETY: EFER accesses are only allowed in kernel mode. We are in kernel mode.
    unsafe {
        let mut msr = Efer::MSR;
        let efer = EferFlags::from_bits_truncate(msr.read()) | EferFlags::NO_EXECUTE_ENABLE;
        msr.write(efer.bits());
    }
}

/// Enables write protection on page entries that do no have the [`WRITABLE`][EntryFlags] flag set.
pub fn enable_wp_bit() {
    // SAFETY: CR0 accesses are only allowed in kernel mode. We are in kernel mode.
    unsafe { Cr0::write(Cr0::read() | Cr0Flags::WRITE_PROTECT) }
}

fn print_cpu_info() {
    // print CPU Vendor
    // SAFETY: cpuid is available and CPUID.0h is then always possible
    let cpuid = unsafe { core::arch::x86_64::__cpuid(0) };
    let ebx = cpuid.ebx;
    let edx = cpuid.edx;
    let ecx = cpuid.ecx;

    let cpu_vendor = [ebx.to_ne_bytes(), edx.to_ne_bytes(), ecx.to_ne_bytes()].concat();
    let cpu_vendor = String::from_utf8(cpu_vendor).unwrap();
    println!("CPU Vendor: {cpu_vendor}");

    // get logical core count per cpu
    // SAFETY: cpuid is available and CPUID.1h is always available
    let cpuid = unsafe { core::arch::x86_64::__cpuid(1) };
    let ebx = cpuid.ebx;

    let logic_cpus = ebx & bitmask!(23..16);
    println!("cpus (logical): {logic_cpus}");

    // get number of cpu cores when vendor is AuthenticAMD
    if cpu_vendor == "AuthenticAMD" {
        // SAFETY: cpuid is available and CPUID.8000_0008h is always available
        let cpuid = unsafe { core::arch::x86_64::__cpuid(0x8000_0008) };
        let ecx = cpuid.ecx;

        let cores = ecx & bitmask!(7..0);

        println!("cores: {cores}, [ecx]: {ecx:#b}");
    }
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
pub fn exit_qemu(exit_code: QemuExitCode) -> ! {
    use x86_64::instructions::port::Port;

    let mut port = Port::new(0xf4);
    // SAFETY: this is used to exit qemu. Any memory violations here will not cause any problem.
    unsafe {
        port.write(exit_code as u32);
    }

    hlt_forever()
}

pub fn test_panic_handler(info: &PanicInfo) -> ! {
    serial_println!("[failed]\n");
    serial_println!("{}\n", info);
    exit_qemu(QemuExitCode::Fail)
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
