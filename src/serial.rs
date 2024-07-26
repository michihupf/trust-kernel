use lazy_static::lazy_static;
use spin::Mutex;
use uart_16550::SerialPort;

lazy_static! {
    /// Static reference to first serial port
    pub static ref SERIAL1: Mutex<SerialPort> = {
        // Safety: serial interface 1 is located at 0x3f8.
        let mut serial = unsafe { SerialPort::new(0x3f8) };
        serial.init();
        Mutex::new(serial)
    };
}

/// Prints a formatted string to the first serial port using the global `SERIAL1`.
#[doc(hidden)]
pub fn _print(args: ::core::fmt::Arguments) {
    use core::fmt::Write;
    use x86_64::instructions::interrupts;

    interrupts::without_interrupts(|| {
        SERIAL1
            .lock()
            .write_fmt(args)
            .expect("Printing to serial failed.");
    });
}

/// This macro is used to print to the first serial port interface.
///
/// Useful for testing purposes where the serial connection
/// can be sent to stdout by the host.
///
/// # Examples
/// ```
/// serial_print!("123"); // prints 123 to the first serial port interface.
/// serial_print!("{}", 123);
///
/// let a = 5;
/// serial_print!("{a}"); // prints 5
/// ```
/// The above code will produce the following output
/// ```
/// 123123
/// 5
/// ```
#[macro_export]
#[allow(clippy::module_name_repetitions)]
macro_rules! serial_print {
    ($($arg:tt)*) => ($crate::serial::_print(format_args!($($arg)*)));
}

/// This macro is used to print to the first serial port interface.
///
/// A newline is appended at the end. Usage is analogous to [`serial_print!`].
///
/// # Examples
/// ```
/// serial_println!("123"); // prints 123 to the first serial port interface.
/// serial_println!("{}", 123);
///
/// let a = 5;
/// serial_println!("{a}"); // prints 5
/// ```
/// The above code will produce the following output
/// ```
/// 123123
/// 5
/// ```
#[macro_export]
#[allow(clippy::module_name_repetitions)]
macro_rules! serial_println {
    () => ($crate::serial_print!("\n"));
    ($fmt:expr) => ($crate::serial_print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::serial_print!(concat!($fmt, "\n"), $($arg)*));
}
