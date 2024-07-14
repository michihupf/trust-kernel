use crate::{print, println};
use lazy_static::lazy_static;
use x86_64::{
    registers::segmentation::Segment,
    structures::{
        gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector},
        tss::TaskStateSegment,
    },
    VirtAddr,
};

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

lazy_static! {
    static ref TSS: TaskStateSegment = {
        let mut tss = TaskStateSegment::new();
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
            const STACK_SIZE: usize = 4096 * 5;
            static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

            #[allow(static_mut_refs)]
            let start = VirtAddr::from_ptr(unsafe { &STACK });
            start + STACK_SIZE // return the top of the stack, as x86 stack grows downwards
        };
        tss
    };
}

struct Selectors {
    code_selector: SegmentSelector,
    tss_selector: SegmentSelector,
}

lazy_static! {
    static ref GDT: (GlobalDescriptorTable, Selectors) = {
        let mut gdt = GlobalDescriptorTable::new();
        let code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
        let tss_selector = gdt.add_entry(Descriptor::tss_segment(&TSS));
        (
            gdt,
            Selectors {
                code_selector,
                tss_selector,
            },
        )
    };
}

// Initializes the Global Descriptor Table (GDT).
pub fn init() {
    use x86_64::instructions::segmentation::CS;
    use x86_64::instructions::tables::load_tss;

    print!("Initializing GDT... ");
    GDT.0.load();
    println!("[ok]");
    print!("Trying to set selectors... ");
    unsafe {
        CS::set_reg(GDT.1.code_selector);
        load_tss(GDT.1.tss_selector);
    }
    println!("[ok]");
}
