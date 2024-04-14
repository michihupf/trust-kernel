#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(trust::test_runner)]
#![reexport_test_harness_main = "test_main"]
#![allow(clippy::empty_loop)]

use core::panic::PanicInfo;
use trust::println;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    test_main();

    // CPU never halts because qemu is exited before
    trust::hlt_forever();
}

#[panic_handler]
pub fn panic(info: &PanicInfo) -> ! {
    trust::test_panic_handler(info);
}

#[test_case]
fn test_println() {
    println!("println! test");
}
