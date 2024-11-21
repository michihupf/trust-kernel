#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

use core::panic::PanicInfo;
use lazy_static::lazy_static;
use multiboot2::{BootInformation, BootInformationHeader};
use spin::Once;
use trust::{
    exit_qemu,
    memory::{self, MemoryController},
    serial_print, serial_println, test_panic_handler,
};
use x86_64::{
    instructions::tables::load_tss,
    registers::segmentation::{Segment, CS},
    structures::{
        gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector},
        idt::{InterruptDescriptorTable, InterruptStackFrame},
        tss::TaskStateSegment,
    },
    VirtAddr,
};

const TEST_DOUBLE_FAULT_IST_INDEX: u16 = 0;

static TEST_TSS: Once<TaskStateSegment> = Once::new();
static TEST_GDT: Once<GlobalDescriptorTable> = Once::new();

trust::entry_asm!();

#[no_mangle]
pub extern "C" fn kernel_entrypoint(mbi_ptr: usize) -> ! {
    serial_print!("Testing stack overflow...\t");

    // Safety: mbi placed in by multiboot2 bootloader
    let mbi = unsafe { BootInformation::load(mbi_ptr as *const BootInformationHeader).unwrap() };

    let mut memory_controller = memory::init(&mbi);
    init_test_idt(&mut memory_controller);

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
                .set_stack_index(TEST_DOUBLE_FAULT_IST_INDEX);
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
}

fn init_test_idt(memory_controller: &mut MemoryController) {
    let double_fault_stack = memory_controller
        .alloc_stack(1)
        .expect("double fault stack allocation failed");

    let tss = TEST_TSS.call_once(|| {
        let mut tss = TaskStateSegment::new();
        tss.interrupt_stack_table[TEST_DOUBLE_FAULT_IST_INDEX as usize] =
            VirtAddr::new(double_fault_stack.top() as u64);
        tss
    });

    let mut code_selector = SegmentSelector(0);
    let mut tss_selector = SegmentSelector(0);
    let gdt = TEST_GDT.call_once(|| {
        let mut gdt = GlobalDescriptorTable::new();
        code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
        tss_selector = gdt.add_entry(Descriptor::tss_segment(tss));
        gdt
    });
    gdt.load();

    // Safety: The given segment selectors point to the correct location.
    unsafe {
        // reload code segment register
        CS::set_reg(code_selector);
        // load TSS
        load_tss(tss_selector);
    }

    TEST_IDT.load();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    test_panic_handler(info);
}

#[allow(unconditional_recursion)]
fn stack_overflow() {
    stack_overflow(); // infinitely recurse
    const VAL: i32 = 0;
    // SAFETY: safe as we know the value exists.
    unsafe {
        core::ptr::read_volatile(&VAL); // prevent tail recursion optimization
    }
}
