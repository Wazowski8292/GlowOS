use alloc::{vec::Vec, string::String};

const COMMAND_AMOUNT: usize = 20;
const SCROLL_BUFFER_HEIGHT: usize = 1000;
const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;

#[macro_export]
macro_rules! write_byte {
    ($args:expr) => ($crate::vga_buffer::WRITER.lock().write_byte($args));
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::vga_buffer::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[macro_export]
macro_rules! get_words {
    () => {
        $crate::vga_buffer::WRITER.lock().get_words(None)
    };
}

#[macro_export]
macro_rules! backspace {
    () => {
        $crate::vga_buffer::WRITER.lock().remove()
    };
}

#[macro_export]
macro_rules! clear_screen {
    () => {
        $crate::vga_buffer::WRITER.lock().clear_screen()
    };
}

#[macro_export]
macro_rules! set_color {
    ($args:expr) => {
        $crate::vga_buffer::WRITER.lock().set_color($args);
    };
}

#[macro_export]
macro_rules! update_color {
    () => {
        $crate::vga_buffer::WRITER.lock().update_color();
    };
}

#[macro_export]
macro_rules! scroll_up {
    () => {
        $crate::vga_buffer::WRITER.lock().scroll_up();
    };
}

#[macro_export]
macro_rules! scroll_down {
    () => {
        $crate::vga_buffer::WRITER.lock().scroll_down();
    };
}

#[macro_export]
macro_rules! last_line {
    () => {
        $crate::vga_buffer::WRITER.lock().last_line();
    };
}

#[macro_export]
macro_rules! get_older_cmd {
    () => {
        $crate::vga_buffer::WRITER.lock().get_older_cmd();
    };
}

#[macro_export]
macro_rules! get_younger_cmd {
    () => {
        $crate::vga_buffer::WRITER.lock().get_younger_cmd();
    };
}

#[macro_export]
macro_rules! reset_command_offset {
    () => {
        $crate::vga_buffer::WRITER.lock().reset_command_offset();
    };
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    if let Some(mut writer) = WRITER.try_lock() {
        writer.write_fmt(args).ok();
    };
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}

impl Color {
    fn from_str(value: &str) -> Result<Self, ()> {
        match value {
            "black" => Ok(Color::Black),
            "blue" => Ok(Color::Blue),
            "green" => Ok(Color::Green),
            "cyan" => Ok(Color::Cyan),
            "red" => Ok(Color::Red),
            "magenta" => Ok(Color::Magenta),
            "brown" => Ok(Color::Brown),
            "lightGray" => Ok(Color::LightGray),
            "darkGray" => Ok(Color::DarkGray),
            "lightBlue" => Ok(Color::LightBlue),
            "lightGreen" => Ok(Color::LightGreen),
            "lightCyan" => Ok(Color::LightCyan),
            "lightRed" => Ok(Color::LightRed),
            "pink" => Ok(Color::Pink),
            "yellow" => Ok(Color::Yellow),
            "white" => Ok(Color::White),
            _ => Err(()),
        }
    }
    pub fn from_u8(value: u8) -> Result<Self, ()> {
        match value {
            0 => Ok(Color::Black),
            1 => Ok(Color::Blue),
            2 => Ok(Color::Green),
            3 => Ok(Color::Cyan),
            4 => Ok(Color::Red),
            5 => Ok(Color::Magenta),
            6 => Ok(Color::Brown),
            7 => Ok(Color::LightGray),
            8 => Ok(Color::DarkGray),
            9 => Ok(Color::LightBlue),
            10 => Ok(Color::LightGreen),
            11 => Ok(Color::LightCyan),
            12 => Ok(Color::LightRed),
            13 => Ok(Color::Pink),
            14 => Ok(Color::Yellow),
            15 => Ok(Color::White),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct ColorCode(u8);

impl ColorCode {
    fn new(foreground: Color, background: Color) -> ColorCode {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct ScreenChar {
    ascii_character: u8,
    color_code: ColorCode,
}



use volatile::Volatile;

#[repr(transparent)]
struct Buffer {
    chars: [[Volatile<ScreenChar>; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

pub struct Writer {
    column_position: usize,
    color_code: ColorCode,
    buffer: &'static mut Buffer,
    scroll_buffer: [[ScreenChar; BUFFER_WIDTH]; SCROLL_BUFFER_HEIGHT],
    scroll_offset: usize,
    command_buffer: [[ScreenChar; BUFFER_WIDTH]; COMMAND_AMOUNT],
    command_offset: usize,
    command_added: usize,
}

impl Writer {
    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            b'\t' => self.tab(),
            _ => {
                if self.column_position >= BUFFER_WIDTH {
                    self.new_line();
                }

                let col = self.column_position;

                let color_code = self.color_code;
                self.scroll_buffer[SCROLL_BUFFER_HEIGHT - 1][col] = ScreenChar {
                    ascii_character: byte,
                    color_code,
                };
                self.column_position += 1;
                self.move_cursor();
            }
        }
    }
    fn tab(&mut self) {
        self.write_string("    ");
    }
    fn new_line(&mut self) {
        for row in 1..SCROLL_BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH {
                let character = self.scroll_buffer[row][col];
                self.scroll_buffer[row - 1][col] = character;
            }
        }
        self.clear_row(SCROLL_BUFFER_HEIGHT - 1);
        self.column_position = 0;
        self.move_cursor()
    }
    fn clear_row(&mut self, row: usize) {
        let blank = ScreenChar {
            ascii_character: b' ',
            color_code: self.color_code,
        };
        for col in 0..BUFFER_WIDTH {
            self.scroll_buffer[row][col] = blank;
        }
        self.move_cursor();
    }
    pub fn clear_screen(&mut self){
        for row in 0..SCROLL_BUFFER_HEIGHT {
            self.clear_row(row);
        }
    }
    pub fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            match byte {
                0x20..=0x7e | b'\n' | b'\t' => self.write_byte(byte),
                _ => self.write_byte(0xfe),
            }
        }
        self.redraw();
    }
    fn get_row(&mut self, row: usize) -> [u8; BUFFER_WIDTH] {
        let mut result = [b' '; BUFFER_WIDTH];
        for col in 0..BUFFER_WIDTH {
            result[col] = self.scroll_buffer[row][col].ascii_character;
        }

        if result[0] == b'$' {
            self.new_command();
        }

        result
    }
    pub fn get_words(&mut self, amount: Option<usize>) -> Vec<String> {
        let search_row = amount.unwrap_or(SCROLL_BUFFER_HEIGHT - 2);
        let row = self.get_row(search_row);

        let len = row.iter()
            .position(|&c| c == b'\n')
            .unwrap_or(row.len());

        let cmd = unsafe {
            core::str::from_utf8_unchecked(&row[..len])
        };

        let mut args = Vec::new();
        let mut current = String::new();
        let mut in_quotes = false;

        for c in cmd.chars() {
            match c {
                '"' => {
                    in_quotes = !in_quotes;
                }
                ' ' if !in_quotes => {
                    if !current.is_empty() {
                        args.push(current.clone());
                        current.clear();
                    }
                }
                _ => {
                    current.push(c);
                }
            }
        }

        if !current.is_empty() {
            args.push(current);
        }

        args
    }
    pub fn remove(&mut self) {
        if self.column_position <  1{
            return;
        }
        let blank = ScreenChar {
            ascii_character: b' ',
            color_code: self.color_code,
        };
        self.column_position -= 1;
        self.scroll_buffer[SCROLL_BUFFER_HEIGHT - 1][self.column_position] = blank;
        self.move_cursor();
        self.redraw();
    }
    fn move_cursor(&mut self) {
        let position = (((SCROLL_BUFFER_HEIGHT - self.scroll_offset - 1) * BUFFER_WIDTH) + self.column_position) as u16;

        unsafe {
            let mut index_register = Port::<u8>::new(0x3D4);
            let mut data_register = Port::<u8>::new(0x3D5);

            // 1. Send High Byte of position
            index_register.write(0x0Eu8);
            data_register.write(((position >> 8) & 0xFF) as u8);

            // 2. Send Low Byte of position
            index_register.write(0x0Fu8);
            data_register.write((position & 0xFF) as u8);
        }
    }
    pub fn set_color(&mut self, cmd: Vec<String>){
        let mut text: [Color; 2] = [
                Color::from_u8(self.color_code.0 & 0xFF).unwrap_or(Color::Yellow), 
                Color::from_u8((self.color_code.0 >> 4 ) & 0xFF).unwrap_or(Color::Black)
            ];

        for i in 1..=2{
            match Color::from_str(cmd[i].to_lowercase().as_str()){
                Ok(color) => {text[i - 1] = color;},
                Err(_) => {
                    self.write_string("Failed to understand the text color arg: ");
                    self.write_string(cmd[i].as_str());
                    self.write_string("\n");
                },
            }
            if cmd.len() == 2 { break;}
        }

        self.color_code = ColorCode::new(text[0], text[1]);
        if cmd.len() == 3 {self.update_bg()}
    }
    pub fn update_bg(&mut self) {
        let new_bg_color = Color::from_u8(self.color_code.0 >> 4 & 0xFF).unwrap_or(Color::Black); 

        for row in 0..SCROLL_BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH {
                let old_char = self.scroll_buffer[row][col];
                
                let original_fg_color = Color::from_u8(old_char.color_code.0 & 0xFF).unwrap_or(Color::Yellow); 
                
                let letter = ScreenChar {
                    ascii_character: old_char.ascii_character,
                    color_code: ColorCode::new(original_fg_color, new_bg_color),
                };
                
                self.scroll_buffer[row][col] = letter;
            }
        }
    }
    pub fn update_color(&mut self) {
        for row in 0..SCROLL_BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH {
                let old_char = self.scroll_buffer[row][col];
                
                let letter = ScreenChar {
                    ascii_character: old_char.ascii_character,
                    color_code: self.color_code,
                };
                
                self.scroll_buffer[row][col] = letter;
            }
        }
    }
    pub fn scroll_up(&mut self) {
        self.scroll_offset -= 1; 
        self.move_cursor();
        self.redraw();
    }
    pub fn scroll_down(&mut self) {
        self.scroll_offset += 1; 
        self.move_cursor();
        self.redraw();
    }
    fn redraw(&mut self) {
        let top = self.scroll_offset;

        for display_row in 0..BUFFER_HEIGHT {
            let virtual_row = top + display_row;
            for col in 0..BUFFER_WIDTH {
                let ch = if virtual_row < SCROLL_BUFFER_HEIGHT {
                    self.scroll_buffer[virtual_row]
                } else {
                    [ScreenChar { ascii_character: b' ', color_code: self.color_code }; BUFFER_WIDTH]
                };
                self.buffer.chars[display_row][col].write(ch[col]);
            }
        }
    }
    pub fn last_line(&mut self) {
        if self.scroll_offset < SCROLL_BUFFER_HEIGHT - BUFFER_HEIGHT {
            self.scroll_offset = SCROLL_BUFFER_HEIGHT - BUFFER_HEIGHT;
        }
    }
    fn new_command(&mut self) {
        for row in (2..COMMAND_AMOUNT).rev() {
            self.command_buffer[row] = self.command_buffer[row - 1]
        }
        self.command_added += 1;
        self.command_buffer[1] = self.scroll_buffer[SCROLL_BUFFER_HEIGHT - 2];
    }
    fn get_command(&mut self) {
        if self.command_offset == 0 {
            return;
        }

        self.scroll_buffer[SCROLL_BUFFER_HEIGHT - 1] = self.command_buffer[self.command_offset - 1];
        self.column_position = 1;
        for n in self.get_words(Some(SCROLL_BUFFER_HEIGHT - 1)).iter() {
            self.column_position += n.len();
        }
        self.column_position = self.column_position.min(BUFFER_WIDTH);

        self.move_cursor();
        self.redraw();
    }
    pub fn get_older_cmd(&mut self) {
        if self.command_offset < COMMAND_AMOUNT.min(self.command_added) {
            self.command_offset +=1;

            self.get_command();
        }
    }
    pub fn get_younger_cmd(&mut self) {
        if self.command_offset > 0 {
            self.command_offset -= 1;

            self.get_command();
        }
    }
    pub fn reset_command_offset(&mut self) {
        self.command_offset = 0;
    }
}

use core::fmt;

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

use lazy_static::lazy_static;
use spin::Mutex;

lazy_static! {
    pub static ref WRITER: Mutex<Writer> = Mutex::new(Writer {
        column_position: 0,
        color_code: ColorCode::new(Color::Yellow, Color::Black),
        buffer: unsafe { &mut *(0xb8000 as *mut Buffer) },
        scroll_buffer: [[ScreenChar {
            ascii_character: b' ',
            color_code: ColorCode::new(Color::Yellow, Color::Black),
        }; BUFFER_WIDTH]; SCROLL_BUFFER_HEIGHT],
        scroll_offset : SCROLL_BUFFER_HEIGHT - BUFFER_HEIGHT,
        command_buffer: [[ScreenChar {
            ascii_character: b' ',
            color_code: ColorCode::new(Color::Yellow, Color::Black),
        }; BUFFER_WIDTH]; COMMAND_AMOUNT],
        command_offset: 0,
        command_added: 1,
    });
}

#[test_case]
fn test_println_simple() {
    println!("test_println_simple output");
}

#[test_case]
fn test_println_many() {
    for _ in 0..200 {
        println!("test_println_many output");
    }
}

#[test_case]
fn test_println_output() {
    use core::fmt::Write;
    use x86_64::instructions::interrupts;

    let s = "Some test string that fits on a single line";
    interrupts::without_interrupts(|| {
        let mut writer = WRITER.lock();
        writeln!(writer, "\n{}", s).expect("writeln failed");
        for (i, c) in s.chars().enumerate() {
            let screen_char = writer.scroll_buffer[SCROLL_BUFFER_HEIGHT - 2][i];
            assert_eq!(char::from(screen_char.ascii_character), c);
        }
    });
}

use x86_64::instructions::port::Port;
