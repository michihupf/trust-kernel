#![no_std]
#![no_main]
#![allow(clippy::empty_loop)]

use core::panic::PanicInfo;

use rustos::{exit_qemu, serial_print, serial_println, QemuExitCode};

#[no_mangle]
pub extern "C" fn _start() -> ! {
    wrong_assertion();
    serial_println!("[no panic]");
    exit_qemu(QemuExitCode::Fail);

    // CPU never halts because we exit qemu before
    rustos::hlt_forever();
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    serial_println!("\r[ok] should_panic::wrong_assertion");
    exit_qemu(rustos::QemuExitCode::Success);

    // CPU never halts because we exit qemu before
    rustos::hlt_forever();
}

fn wrong_assertion() {
    serial_print!("should_panic::wrong_assertion...\t");
    assert_eq!(0, 1);
}
