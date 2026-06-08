use crate::renderer::{text_font::SYS_FONT, Color, RENDERER};
use alloc::vec::Vec;
use alloc::vec;

const CHAR_SIZE: usize = 8;

#[derive(Clone, Copy)]
struct Letter {
    ascii_character: u8,
    color: Color,
}

pub struct FontRenderer {
    buffer: Vec<Letter>,
    scale: usize,
    max_chars_x: usize,
    max_chars_y: usize,
}

impl FontRenderer {
    pub fn new(max_chars_x: usize, max_chars_y: usize) -> Self{
        let white = Color::new(255, 255, 255);
        let scale = 2;
        let max_x = (max_chars_x / CHAR_SIZE / scale) as usize;
        let max_y = (max_chars_y / CHAR_SIZE / scale) as usize;

        let letter = Letter {
            ascii_character: 0,
            color: white,
        };
        Self {
            buffer: vec![letter; max_x * max_y],
            scale: scale,
            max_chars_x: max_x,
            max_chars_y: max_y,
        }
    }

    fn get(&self, col: usize, row: usize) -> Letter {
        self.buffer[row * self.max_chars_x + col]
    }

    fn set(&mut self, col: usize, row: usize, letter: Letter) {
        self.buffer[row * self.max_chars_x + col] = letter;
    }

    fn draw_char(&self, x_pos: usize, y_pos: usize, letter: Letter) {
        #[allow(static_mut_refs)]
        let renderer = unsafe { RENDERER.as_mut().unwrap() };
        let bitmap: u64 = SYS_FONT[letter.ascii_character as usize];

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
    
    pub fn test(&mut self) {
        let white = Color::new(255, 255, 255);

        for hex in 0..SYS_FONT.len().min(self.max_chars_x * self.max_chars_y) {
            let letter = Letter {
                ascii_character: hex as u8,
                color: white.clone(),
            };
            let col = hex % self.max_chars_x;
            let row = (hex / self.max_chars_x) as usize; 
            self.set(col, row, letter); 
        }
    }
}