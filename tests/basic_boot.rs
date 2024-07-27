#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(trust::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;

use trust::println;

trust::entry_asm!();

#[no_mangle]
pub extern "C" fn kernel_main() -> ! {
    test_main();

    // CPU never halts because qemu is exited before
    trust::hlt_forever();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    trust::test_panic_handler(info);
}

#[test_case]
fn test_println() {
    println!("println! test");
}
