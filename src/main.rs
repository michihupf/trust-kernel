#![feature(custom_test_frameworks)]
#![test_runner(trust::test_runner)]
#![reexport_test_harness_main = "trust::test_main"]
#![no_std]
#![no_main]

trust::entry_asm!();

#[no_mangle]
pub extern "C" fn kentry(_mbi_ptr: usize) -> ! {
    #[cfg(not(test))]
    trust::kernel_main(_mbi_ptr);

    #[cfg(test)]
    trust::exit_qemu(trust::QemuExitCode::Success)
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
