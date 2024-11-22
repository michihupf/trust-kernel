#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(trust::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;

use alloc::boxed::Box;
use multiboot2::{BootInformation, BootInformationHeader};
use trust::{hlt_forever, memory};

extern crate alloc;

trust::entry_asm!();

#[no_mangle]
pub extern "C" fn kentry(mbi_ptr: usize) -> ! {
    // Safety: mbi is placed here by mutliboot2 bootloader
    let mbi = unsafe { BootInformation::load(mbi_ptr as *const BootInformationHeader).unwrap() };

    let _memory_controller = memory::init(&mbi);

    test_main();

    hlt_forever();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    trust::test_panic_handler(info);
}

#[test_case]
fn box_alloc() {
    let v1 = Box::new(123);
    let v2 = Box::new(321);
    assert_eq!(*v1, 123);
    assert_eq!(*v2, 321);
}

#[test_case]
fn vec_alloc() {
    use alloc::vec::Vec;
    let n = 1000;
    let mut vec = Vec::new();
    for i in 1..=n {
        vec.push(i);
    }
    assert_eq!(vec.iter().sum::<u64>(), n * (n + 1) / 2);
}

#[test_case]
fn reuse_after_free() {
    for i in 0..memory::HEAP_SIZE {
        let x = Box::new(i);
        assert_eq!(*x, i);
    }
}

#[test_case]
fn prolonged_use() {
    let prolonged_lifetime = Box::new(123);
    for i in 0..memory::HEAP_SIZE {
        let x = Box::new(i);
        assert_eq!(*x, i);
    }
    assert_eq!(*prolonged_lifetime, 123);
}
