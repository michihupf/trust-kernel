#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(trust::test_runner)]
#![reexport_test_harness_main = "test_main"]

use alloc::boxed::Box;
use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;
use trust::{heap, hlt_forever, memory};
use x86_64::VirtAddr;

extern crate alloc;

entry_point!(main);

fn main(boot_info: &'static BootInfo) -> ! {
    // initialize IDT, GDT and enable external interrupts
    trust::init_basics();

    // initialize paging
    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator =
        unsafe { memory::BootInfoFrameAllocator::init(&boot_info.memory_map) };

    // initialize heap
    heap::init(&mut mapper, &mut frame_allocator).expect("heap initialization failed.");

    test_main();

    hlt_forever();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    trust::test_panic_handler(info)
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
    for i in 0..heap::HEAP_SIZE {
        let x = Box::new(i);
        assert_eq!(*x, i);
    }
}

#[test_case]
fn prolonged_use() {
    let prolonged_lifetime = Box::new(123);
    for i in 0..heap::HEAP_SIZE {
        let x = Box::new(i);
        assert_eq!(*x, i);
    }
    assert_eq!(*prolonged_lifetime, 123);
}
