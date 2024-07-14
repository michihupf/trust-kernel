#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

use core::panic::PanicInfo;
use lazy_static::lazy_static;
use trust::{exit_qemu, gdt::DOUBLE_FAULT_IST_INDEX, serial_print, serial_println};
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

#[no_mangle]
pub extern "C" fn _start() -> ! {
    serial_print!("Testing stack overflow...\t");

    trust::gdt::init();
    init_test_idt();

    // trigger stack overflow
    stack_overflow();

    panic!("Continued after stack overflow!");
}

lazy_static! {
    static ref TEST_IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        unsafe {
            idt.double_fault
                .set_handler_fn(double_fault_handler)
                .set_stack_index(DOUBLE_FAULT_IST_INDEX);
        }
        idt
    };
}

extern "x86-interrupt" fn double_fault_handler(
    _stack_frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    serial_println!("\r[ok] stack overflow test ");
    exit_qemu(trust::QemuExitCode::Success);

    // CPU never halts because we exit qemu before
    trust::hlt_forever();
}

fn init_test_idt() {
    TEST_IDT.load();
}

#[allow(unconditional_recursion)]
fn stack_overflow() {
    stack_overflow(); // infinitely recurse
    const VAL: i32 = 0;
    // UNSAFE: safe as we know the value exists.
    unsafe {
        core::ptr::read_volatile(&VAL); // prevent tail recursion optimization
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    trust::test_panic_handler(info);
}
