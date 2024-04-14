#![no_std]
#![no_main]
// set up testing
#![feature(custom_test_frameworks)]
#![test_runner(trust::test_runner)]
#![reexport_test_harness_main = "test_main"] // test main function should be renamed from main() because of no_main
#![allow(clippy::empty_loop)]

extern crate alloc;

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;
use trust::{
    heap, memory, println,
    task::{executor::Executor, keyboard, Task},
};
use x86_64::{structures::paging::Page, VirtAddr};

entry_point!(kernel_main);

/// This is the kernel entry point. It is called by the bootloader.
fn kernel_main(boot_info: &'static BootInfo) -> ! {
    // kernel entry point

    // print "Booting" to the screen
    println!("Booting tRust...");

    // initialize GDT, IDT and enable external interrupts
    trust::init();

    // initialize paging
    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator =
        unsafe { memory::BootInfoFrameAllocator::init(&boot_info.memory_map) };

    // initialize heap
    heap::init(&mut mapper, &mut frame_allocator).expect("heap initialization failed.");

    // map an unused page
    let page = Page::containing_address(VirtAddr::new(0xdeadbeef));
    memory::create_example_mapping(page, &mut mapper, &mut frame_allocator);

    // write "New!" to the screen though memory mapping
    let page_ptr: *mut u64 = page.start_address().as_mut_ptr();
    unsafe { page_ptr.offset(200).write_volatile(0xf021_f077_f065_f04e) };

    // run tests when in test config
    #[cfg(test)]
    test_main();

    // test asynchronous tasks
    let mut executor = Executor::new();
    executor.spawn(Task::new(print_async()));
    executor.spawn(Task::new(keyboard::print_keypresses()));
    executor.run();
}

async fn async_num() -> u32 {
    69420
}

async fn print_async() {
    let number = async_num().await;
    println!("async print: {}", number);
}

/// This function is called on panic and prints information to VGA text buffer.
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    use trust::hlt_forever;

    println!("{}", info);
    hlt_forever();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    trust::test_panic_handler(info);
}
