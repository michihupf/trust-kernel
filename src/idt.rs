use crate::{gdt, hlt_forever, print, println};
#[allow(unused_imports)]
use core::arch::asm;
use lazy_static::lazy_static;
use pic8259::ChainedPics;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        // Exceptions
        idt.divide_error.set_handler_fn(div_by_zero_handler);
        idt.debug.set_handler_fn(debug_handler);
        idt.non_maskable_interrupt.set_handler_fn(non_maskable_interrupt_handler);
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.overflow.set_handler_fn(overflow_handler);
        idt.bound_range_exceeded.set_handler_fn(bound_range_exceeded_handler);
        idt.invalid_opcode.set_handler_fn(invalid_opcode_handler);
        idt.device_not_available.set_handler_fn(device_not_available_handler);
        unsafe {
            idt.double_fault
                .set_handler_fn(double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
        }
        // TODO: Invalid TSS
        // TODO: Segment Not Present
        // TODO: Stack-Segment Fault
        // TODO: General Protection Fault
        idt.page_fault.set_handler_fn(page_fault_handler);
        // TODO: x87 Floating-Point Exception
        // TODO: Alignment Check
        // TODO: Machine Check
        // TODO: SIMD Floating-Point Exception <-- low priority as SIMD is not enabled for kernel
        // TODO: Virtualization Exception
        // TODO: Control Protection Exception
        // TODO: Hypervisor Injection Exception
        // TODO: VMM Communication Exception
        // TODO: Security Exception

        // PIC 8259 Hardware Interrupts

        // Intel 8253 timer interrupt handler
        idt[InterruptIndex::Timer.as_usize()].set_handler_fn(timer_interrupt_handler);
        // PS/2 Keyboard interrupt handler
        idt[InterruptIndex::Keyboard.as_usize()].set_handler_fn(keyboard_interrupt_handler);

        idt
    };
}

pub fn init() {
    print!("Initializing IDT... ");
    IDT.load();
    println!("[ok]");
}

/// Exception handler for a division by zero exception.
extern "x86-interrupt" fn div_by_zero_handler(stack_frame: InterruptStackFrame) {
    println!("CPU EXCEPTION: DIVISION BY ZERO\n{:#?}", stack_frame);
}

#[test_case]
fn test_div_by_zero_exception() {
    // invoke a division by zero exception by invoking a 0x0 software interrupt.
    unsafe {
        x86_64::software_interrupt!(0x0);
    }
}

/// Exception handler for a debug exception.
extern "x86-interrupt" fn debug_handler(stack_frame: InterruptStackFrame) {
    println!("CPU EXCEPTION: DEBUG\n{:#?}", stack_frame);
}

#[test_case]
fn test_debug_exception() {
    // invoke a debug exception by invoking a 0x1 software interrupt.
    unsafe {
        x86_64::software_interrupt!(0x1);
    }
}

/// Exception handler for a non-maskable interrupt.
extern "x86-interrupt" fn non_maskable_interrupt_handler(stack_frame: InterruptStackFrame) {
    println!("CPU EXCEPTION: NMI\n{:#?}", stack_frame);
}

#[test_case]
fn test_non_maskable_interrupt() {
    unsafe {
        x86_64::software_interrupt!(0x2);
    }
}

/// Exception handler for a breakpoint exception (INT3).
extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    println!("CPU EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

#[test_case]
fn test_breakpoint_execption() {
    // invoke breakpoint exception
    x86_64::instructions::interrupts::int3();
}

/// Exception handler for an overflow exception.
extern "x86-interrupt" fn overflow_handler(stack_frame: InterruptStackFrame) {
    println!("CPU EXCEPTION: OVERFLOW\n{:#?}", stack_frame);
}

#[test_case]
fn test_overflow_exception() {
    // invoke an overflow exception by invoking a 0x4 software interrupt.
    unsafe {
        x86_64::software_interrupt!(0x4);
    }
}

/// Exception handler for a bound range exceeded exception.
///
/// # Bound Range Exceeded
/// This error occurs when an array index is out of bounds when checked
/// with the BOUND instruction against and array with lower and upper bound.
extern "x86-interrupt" fn bound_range_exceeded_handler(stack_frame: InterruptStackFrame) {
    println!("CPU EXCEPTION: BOUND RANGE EXCEEDED\n{:#?}", stack_frame);
}

#[test_case]
fn test_bound_range_exceeded() {
    // invoke a 0x5 software interrupt
    unsafe {
        x86_64::software_interrupt!(0x5);
    }
}

/// # Invalid Opcode
/// Occurs when CPU tries to execute an invalid or undefined opcode. Also occurs in case of
///
/// - instruction tries to access a non-existent control register
/// - UD is executed
extern "x86-interrupt" fn invalid_opcode_handler(stack_frame: InterruptStackFrame) {
    println!("CPU EXCEPTION: INVALID OPCODE\n{:#?}", stack_frame);
}

#[test_case]
fn test_invalid_opcode() {
    unsafe {
        x86_64::software_interrupt!(0x6);
    }
}

/// # Device Not Available
/// Occurs when FPU instruction is attempted but no FPU is available. This is not likely on modern systems but
/// the FPU can be disabled using flags in the CR0 register.
extern "x86-interrupt" fn device_not_available_handler(stack_frame: InterruptStackFrame) {
    println!("CPU EXCEPTION: DEVICE NOT AVAILABLE\n{:#?}", stack_frame);
}

#[test_case]
fn test_device_not_available() {
    unsafe {
        x86_64::software_interrupt!(0x7);
    }
}

/// Exception handler for a double fault exception.
extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    panic!("CPU EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
}

/// Exception handler for a page fault exception.
extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    use x86_64::registers::control::Cr2; // CR2 is populated with the accessed address at page fault

    println!("CPU EXCEPTION: PAGE FAULT");
    println!("Accessed Address: {:?}", Cr2::read());
    println!("Error Code: {:?}", error_code);
    println!("{:#?}", stack_frame);

    hlt_forever();
}

// ----------------------------------------------------------------
// Implementation of the PIC8259 hardware interrupts follows below:
// ----------------------------------------------------------------

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

pub static PICS: spin::Mutex<ChainedPics> =
    spin::Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

/// Enum for identification of PIC 8259 interrupt indeces.
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
    Keyboard,
}

impl InterruptIndex {
    fn as_u8(self) -> u8 {
        self as u8
    }

    fn as_usize(self) -> usize {
        usize::from(self.as_u8())
    }
}

/// Interrupt handler for the Intel 8253 timer interrupt.
extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    // use core::sync::atomic::AtomicUsize;

    // static COUNTER: AtomicUsize = AtomicUsize::new(0);
    // let current = COUNTER.fetch_add(1, core::sync::atomic::Ordering::Relaxed) % 78;

    // let mut s: [u8; 80] = [b' '; 80];
    // s[0] = b'[';
    // s[79] = b']';
    // s[current + 1] = b'>';
    // for i in 0..current {
    //     s[i + 1] = b'=';
    // }

    // print!("\r{}", core::str::from_utf8(&s).unwrap());

    // send EOI after successful handling
    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
    }
}

/// Interrupt handler for the PS/2 Keyboard interrupt.
extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    use x86_64::instructions::port::Port;

    // read the scancode from the PS/2 port (0x60)
    let mut port = Port::new(0x60);
    let scancode: u8 = unsafe { port.read() };
    crate::task::keyboard::add_scancode(scancode);

    // send EOI after successful handling
    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
}
