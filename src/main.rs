#![feature(custom_test_frameworks)]
#![test_runner(trust::test_runner)]
#![reexport_test_harness_main = "trust::test_main"]
#![no_std]
#![no_main]

trust::entry_asm!();

/// This is the kernel entry point. It is called by the bootloader.
#[no_mangle]
pub extern "C" fn kernel_main(mbi_ptr: usize) -> ! {
    // kernel entry point
    trust::kernel_main(mbi_ptr) // this somehow doesn't get any #[cfg(test)] treatment when in test config??
}

#[cfg(test)]
mod panic {
    use core::panic::PanicInfo;

    use trust::test_panic_handler;

    #[panic_handler]
    fn panic(info: &PanicInfo) -> ! {
        test_panic_handler(info);
    }
}

#[cfg(not(test))]
mod panic {
    use core::panic::PanicInfo;

    use trust::{hlt_forever, println};

    /// This function is called on panic and prints information to VGA text buffer.
    #[panic_handler]
    fn panic(info: &PanicInfo) -> ! {
        println!("{}", info);
        hlt_forever();
    }
}
