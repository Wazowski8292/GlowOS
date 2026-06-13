use crate::renderer::{text_font::{SYS_FONT, char_to_font_index}, Color, RENDERER};
use alloc::vec::Vec;
use alloc::vec;

const CHAR_SIZE: usize = 8;
const DEFAULT_LETTER: Letter = Letter {
    ascii_character : ' ',
    color: Color { r: 255, g: 255, b: 255},
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
}

impl FontRenderer {
    pub fn new(max_chars_x: usize, max_chars_y: usize) -> Self{
        let scale = 2;
        let max_x = (max_chars_x / CHAR_SIZE / scale) as usize;
        let max_y = (max_chars_y / CHAR_SIZE / scale) as usize;

        Self {
            buffer: vec![DEFAULT_LETTER; max_x * max_y],
            scale: scale,
            max_chars_x: max_x,
            max_chars_y: max_y,
            x_pos: 0,
            y_pos: 0,
        }
    }

    fn get(&self, col: usize, row: usize) -> Letter {
        self.buffer[row * self.max_chars_x + col]
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
                if (bitmap >> bit) & 1 == 1 {
                    for rel_x in 0..self.scale {
                        for rel_y in 0..self.scale {
                            renderer.put_pixel(
                                x_pos * CHAR_SIZE * self.scale + x * self.scale + rel_x,
                                y_pos * CHAR_SIZE * self.scale + y * self.scale + rel_y,
                                letter.color,
                            );
                        }
                    }
                }
            }
        }
    }

    pub fn draw_buffer(&self)  {
        for i in 0..self.max_chars_x * self.max_chars_y {
            let col = i % self.max_chars_x;
            let row = i / self.max_chars_x;
            self.draw_char(col, row, self.get(col, row));
        }
    }

    fn print_char(&mut self, c: char) {
        let a = Letter {
            ascii_character: c,
            color: Color::new(255, 255, 255),
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
    }

    pub fn backspace(&mut self){
        
        self.x_pos = (self.x_pos - 1).max(0);
        self.buffer[self.x_pos + self.y_pos * self.max_chars_x] = DEFAULT_LETTER;
    }
}

use core::fmt;

impl fmt::Write for FontRenderer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.print_string(s);
        Ok(())
    }
}