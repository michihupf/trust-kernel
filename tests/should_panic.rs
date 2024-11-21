#![no_std]
#![no_main]
#![allow(clippy::empty_loop)]

use core::panic::PanicInfo;
use trust::{exit_qemu, serial_print, serial_println, QemuExitCode};

trust::entry_asm!();

#[no_mangle]
pub extern "C" fn kernel_entrypoint() -> ! {
    wrong_assertion();
    serial_println!("[no panic]");
    exit_qemu(QemuExitCode::Fail);

    // CPU never halts because we exit qemu before
    trust::hlt_forever();
}

#[cfg(test)]
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    serial_println!("\r[ok] should_panic::wrong_assertion");
    exit_qemu(trust::QemuExitCode::Success);

    // CPU never halts because we exit qemu before
    trust::hlt_forever();
}

fn wrong_assertion() {
    serial_print!("should_panic::wrong_assertion...\t");
    assert_eq!(0, 1);
}
