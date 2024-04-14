use core::fmt;
use lazy_static::lazy_static;
use spin::Mutex;
use volatile::Volatile;

/// The Color enum is an abstraction for the 4-bit VGA text buffer colors.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
    Black = 0x0,
    Blue = 0x1,
    Green = 0x2,
    Cyan = 0x3,
    Red = 0x4,
    Magenta = 0x5,
    Brown = 0x6,
    LightGray = 0x7,
    DarkGray = 0x8,
    LightBlue = 0x9,
    LightGreen = 0xa,
    LightCyan = 0xb,
    LightRed = 0xc,
    Pink = 0xd,
    Yellow = 0xe,
    White = 0xf,
}

/// The ColorCode struct serves as an abstraction for a 8-bit VGA text buffer color code
/// formed from the foreground and background color. The blink bit (bit 7) is included in
/// background color.
///
/// The code is constructed from the background color B and the foreground color F in the
/// following way: B << 4 | F.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct ColorCode(u8);

impl ColorCode {
    pub fn new(font: Color, background: Color) -> ColorCode {
        // first 4 bits are foreground, last 4 are background
        ColorCode((background as u8) << 4 | (font as u8))
    }
}

/// A ScreenChar is a C-like struct representation of an ASCII character along with an
/// associated ColorCode that defines the appearence of the character.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct ScreenChar {
    ascii: u8,
    color_code: ColorCode,
}

const BUFFER_SIZE_X: usize = 80;
const BUFFER_SIZE_Y: usize = 25;

#[repr(transparent)]
struct Buffer {
    chars: [[Volatile<ScreenChar>; BUFFER_SIZE_X]; BUFFER_SIZE_Y],
}

/// A writer can be used to modify the VGA text buffer.
pub struct Writer {
    // The position of the cursor in the lowest row.
    column_pos: usize,
    // The ColorCode to be used for subsequent writes.
    color_code: ColorCode,
    // mutable reference to the VGA text buffer (0xb8000).
    buffer: &'static mut Buffer,
}

impl Writer {
    /// Writes a byte to the buffer. Does not check for printable ASCII characters.
    fn write(&mut self, byte: u8) {
        match byte {
            b'\n' => self.newline(),
            b'\r' => self.column_pos = 0,
            0x08 => self.backspace(),
            byte => {
                if self.column_pos >= BUFFER_SIZE_X {
                    self.newline()
                }

                let row = BUFFER_SIZE_Y - 1;
                let col = self.column_pos;

                let color_code = self.color_code;
                self.buffer.chars[row][col].write(ScreenChar {
                    ascii: byte,
                    color_code,
                });
                self.column_pos += 1;
            }
        }
    }

    /// Performs a newline operation on the buffer by moving every row up by 1.
    fn newline(&mut self) {
        // move every character up by one row
        for row in 1..BUFFER_SIZE_Y {
            for col in 0..BUFFER_SIZE_X {
                let char = self.buffer.chars[row][col].read();
                self.buffer.chars[row - 1][col].write(char);
            }
        }
        self.clear_row(BUFFER_SIZE_Y - 1);
    }

    /// Clears the specified row. When used on the last line this is a carridge return (b'\r').
    fn clear_row(&mut self, row: usize) {
        let blank = ScreenChar {
            ascii: b' ',
            color_code: self.color_code,
        };
        for col in 0..BUFFER_SIZE_X {
            self.buffer.chars[row][col].write(blank);
        }
        self.column_pos = 0;
    }

    /// Clears the last written character.
    fn backspace(&mut self) {
        // when row is empty ignore backspace characters
        if self.column_pos == 0 {
            return;
        }

        let blank = ScreenChar {
            ascii: b' ',
            color_code: self.color_code,
        };

        // column of the last typed char
        let col = self.column_pos - 1;
        self.buffer.chars[BUFFER_SIZE_Y - 1][col].write(blank);
        self.column_pos = col;
    }

    // Writes a string to the buffer. Checks for printable ASCII characters.
    fn write_string(&mut self, str: &str) {
        for byte in str.bytes() {
            match byte {
                // check for printable ASCII
                0x20..=0x7e | b'\n' | b'\r'
                | 0x08 /* Backspace (BS)*/ => self.write(byte),
                b'\t' => self.write_string("    "),
                0x7f => self.write_string("<DEL>"),
                // any other non-printable ASCII character - we will limit it to 0x7e
                _ => self.write(0x7e),
            }
        }
    }
}

// Implement format strings for Writer
impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

lazy_static! {
    pub static ref WRITER: Mutex<Writer> = Mutex::new(Writer {
        column_pos: 0,
        color_code: ColorCode::new(Color::White, Color::Black),
        buffer: unsafe { &mut *(0xb8000 as *mut Buffer) },
    });
}

/// Prints a formatted string to the VGA text buffer using the global `WRITER`.
#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    use x86_64::instructions::interrupts;

    interrupts::without_interrupts(|| {
        WRITER.lock().write_fmt(args).unwrap();
    });
}

/// This macro is used to print to the VGA text buffer.
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::vga_buffer::_print(format_args!($($arg)*)));
}

/// This macro is used to print to the VGA text buffer. Newline is appended.
#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

// -- UNIT TESTS -- //

/// Test VGA text buffer print macro.
#[test_case]
fn vga_text_buffer_print() {
    crate::print!("testing print! macro\n");
}

/// Test VGA text buffer println macro.
#[test_case]
fn vga_text_buffer_println() {
    crate::println!("testing println! macro");
}

/// Test VGA text buffer for multiple lines.
#[test_case]
fn vga_text_buffer_many_print() {
    for i in 0..256 {
        crate::println!("{}", i);
    }
}

/// Test VGA text buffer functionality. Fails if content is not displayed correctly
#[test_case]
fn vga_text_buffer_functionality() {
    use crate::vga_buffer::WRITER;
    use core::fmt::Write;
    use x86_64::instructions::interrupts;

    let s = "Content";
    interrupts::without_interrupts(|| {
        let mut writer = WRITER.lock();
        writeln!(writer, "\n{}", s).expect("writeln failed");
        for (i, c) in s.chars().enumerate() {
            let screen_char = writer.buffer.chars[BUFFER_SIZE_Y - 2][i].read();
            assert_eq!(char::from(screen_char.ascii), c);
        }
    });
}

/// Test VGA buffer backspace functionality
#[test_case]
fn vga_text_buffer_backspace() {
    use crate::vga_buffer::WRITER;
    use core::fmt::Write;
    use x86_64::instructions::interrupts;

    let s = "Content";
    interrupts::without_interrupts(|| {
        let mut writer = WRITER.lock();
        write!(writer, "\n{}", s).expect("writeln failed");
        for _ in 0..s.len() {
            writer.write(0x08); // remove every character <BS> = 0x08
        }
        for i in 0..s.len() {
            // check that the i-th character is ' '
            let screen_char = writer.buffer.chars[BUFFER_SIZE_Y - 1][i].read();
            assert_eq!(char::from(screen_char.ascii), ' ');
        }
    });
}
