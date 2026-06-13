use crate::renderer::{text_font::{SYS_FONT, char_to_font_index}, Color, RENDERER};
use alloc::{vec ,vec::Vec, string::String};
use crate::serial_println;

const CHAR_SIZE: usize = 8;
const DEFAULT_LETTER: Letter = Letter {
    ascii_character : ' ',
    color: Color { r: 0, g: 0, b: 0},
};

#[derive(Clone, Copy)]
struct Letter {
    ascii_character: char,
    color: Color,
}

pub struct FontRenderer {
    buffer: Vec<Letter>,
    scale: usize,
    max_chars_x: usize,
    max_chars_y: usize,
    x_pos: usize,
    y_pos: usize,
    background_color: Color,
    font_color: Color,
}

impl FontRenderer {
    pub fn new(max_chars_x: usize, max_chars_y: usize, bg_color: Color) -> Self{
        let scale = 2;
        let max_x = (max_chars_x / CHAR_SIZE / scale) as usize;
        let max_y = (max_chars_y / CHAR_SIZE / scale) as usize;
        let text_color = Color::new(255, 255, 0);

        Self {
            buffer: vec![DEFAULT_LETTER; max_x * max_y],
            scale: scale,
            max_chars_x: max_x,
            max_chars_y: max_y,
            x_pos: 0,
            y_pos: 0,
            background_color: bg_color,
            font_color: text_color,
        }
    }

    fn get(&self, col: usize, row: usize) -> Letter {
        if col < self.max_chars_x || row < self.max_chars_y{
            self.buffer[row * self.max_chars_x + col]
        }
        else {
            DEFAULT_LETTER
        }
    }

    fn set(&mut self, letter: Letter) {
        self.buffer[self.y_pos * self.max_chars_x + self.x_pos] = letter;
        self.x_pos += 1;
        if self.x_pos >= self.max_chars_x {
            self.x_pos = 0;
            self.y_pos += 1;
        }
    }

    fn draw_char(&self, x_pos: usize, y_pos: usize, letter: Letter) {
        #[allow(static_mut_refs)]
        let renderer = unsafe { RENDERER.as_mut().unwrap() };
        let bitmap: u64 = SYS_FONT[char_to_font_index(letter.ascii_character).unwrap_or(0) as usize];

        for y in 0..CHAR_SIZE {
            for x in 0..CHAR_SIZE {
                let bit = y * CHAR_SIZE + x;
                for rel_x in 0..self.scale {
                    for rel_y in 0..self.scale {
                        renderer.put_pixel(
                            x_pos * CHAR_SIZE * self.scale + x * self.scale + rel_x,
                            y_pos * CHAR_SIZE * self.scale + y * self.scale + rel_y,
                            {
                                if (bitmap >> bit) & 1 == 1 {
                                    letter.color
                                } else {
                                    DEFAULT_LETTER.color
                                }});
                    }
                }
            }
        }
    }

    pub fn draw_buffer(&self)  {

        for i in 0..self.max_chars_x * self.max_chars_y {
            let col = i % self.max_chars_x;
            let row = i / self.max_chars_x;
            let letter = self.get(col, row);

            if letter.ascii_character != ' ' {
                self.draw_char(col, row, letter);
            }
        }
    }

    fn print_char(&mut self, c: char) {
        let a = Letter {
            ascii_character: c,
            color: self.font_color,
        };
        self.set(a);
    }

    pub fn print_string(&mut self, msg: &str) {
        for letter in msg.chars() {
            match letter {
                '\n' => {self.y_pos += 1; self.x_pos = 0},
                '\t' => {self.print_string("    ")},
                _ => {self.print_char(letter)},
            }
        }
        self.draw_buffer();
    }

    pub fn backspace(&mut self) {
        self.x_pos -= if self.x_pos == 0 {
            0
        } else {
           1
        };

        self.buffer[self.x_pos + self.y_pos * self.max_chars_x] = DEFAULT_LETTER;
        self.draw_char(self.x_pos, self.y_pos, DEFAULT_LETTER);
    }

    fn parse_line(&self, row: usize) -> Vec<String>{
        let mut args: Vec<char> = Vec::new();
        let mut current: Vec<String> = Vec::new();
        let mut in_quotes = false;

        for col in 0..self.max_chars_x{
            let letter = self.get(col, row).ascii_character;

            
            match letter {
                ' ' => {
                    if in_quotes {continue;}
                    if !args.is_empty() { 
                        current.push(args.iter().collect());
                    }
                    args.clear();
                }
                '"' => {
                    in_quotes = !in_quotes;
                }
                _ => {args.push(letter)}
            }
        }

        if !args.is_empty(){
            current.push(args.iter().collect());
        }

        current
    }

    pub fn parse_last_line(&self) -> Vec<String>{
        self.parse_line(self.y_pos - 1)
    }

    pub fn clear_buffer(&mut self) {
        for i in 0..self.max_chars_x * self.max_chars_y {
            let col = i % self.max_chars_x;
            let row = i / self.max_chars_x;
            let letter = self.get(col, row);

            if letter.ascii_character != ' ' {
                self.buffer[i] = DEFAULT_LETTER;
            }
        }
        self.x_pos = 0;
        self.y_pos = 0;
    }
}

use core::fmt;

impl fmt::Write for FontRenderer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.print_string(s);
        Ok(())
    }
}