use crate::renderer::{text_font::{SYS_FONT, char_to_font_index}, Color, RENDERER};
use alloc::{vec ,vec::Vec, string::String};

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
    max_screen_chars_y: usize,
    x_pos: usize,
    y_pos: usize,
    cursor_x_pos: usize,
    cursor_y_pos: usize,
    draw_cursor_timer: usize,
    max_draw_cursor_timer: usize,
    draw_cursor: bool,
    background_color: Color,
    font_color: Color,
}

impl FontRenderer {
    pub fn new(max_chars_x: usize, max_screen_chars_y: usize, bg_color: Color) -> Self{
        let scale = 2;
        let max_x = (max_chars_x / CHAR_SIZE / scale) as usize;
        let max_y = (max_screen_chars_y / CHAR_SIZE / scale) as usize;
        let text_color = Color::new(255, 255, 0);

        Self {
            buffer: vec![DEFAULT_LETTER; max_x * max_y],
            scale: scale,
            max_chars_x: max_x,
            max_chars_y: max_y,
            max_screen_chars_y: max_y,
            x_pos: 0,
            y_pos: 0,
            cursor_x_pos: 0,
            cursor_y_pos: 0,
            draw_cursor_timer: 0,
            max_draw_cursor_timer: 10,
            draw_cursor: true,
            background_color: bg_color,
            font_color: text_color,
        }
    }

    fn get(&self, col: usize, row: usize) -> Letter {
        if col < self.max_chars_x || row < self.max_screen_chars_y{
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
                                    self.background_color
                                }});
                    }
                }
            }
        }
    }

    pub fn draw_buffer(&self)  {

        for i in 0..self.max_chars_x * self.max_screen_chars_y {
            let col = i % self.max_chars_x;
            let row = i / self.max_chars_x;
            let letter = self.get(col, row);

            if letter.ascii_character != ' ' {
                self.draw_char(col, row, letter);
            }
        }
    }

    fn print_char(&mut self, c: char) {
        let msg = Letter {
            ascii_character: c,
            color: self.font_color,
        };
        self.set(msg);
        self.draw_cursor(self.x_pos, self.y_pos);
        self.draw_char(self.x_pos, self.y_pos, msg);
    }

    pub fn print_string(&mut self, msg: &str) {
        for letter in msg.chars() {
            match letter {
                '\n' => {self.y_pos += 1; self.x_pos = 0; self.draw_cursor(self.x_pos, self.y_pos)},
                '\t' => {self.print_string("    ")},
                _ => {self.print_char(letter)},
            }
        }
    }

    pub fn backspace(&mut self) {
        self.buffer[self.x_pos + self.y_pos * self.max_chars_x] = DEFAULT_LETTER;
        self.draw_char(self.x_pos, self.y_pos, DEFAULT_LETTER);

        self.x_pos -= if self.x_pos == 0 {
            0
        } else {
           1
        };

        self.draw_cursor(self.x_pos, self.y_pos);
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
        for i in 0..self.max_chars_x * self.max_screen_chars_y {
            let col = i % self.max_chars_x;
            let row = i / self.max_chars_x;
            let letter = self.get(col, row);

            if letter.ascii_character != ' ' {
                self.buffer[i] = DEFAULT_LETTER;
            }
        }
        self.x_pos = 0;
        self.y_pos = 0;
        self.draw_cursor(self.x_pos, self.y_pos);
    }
    
    fn clear_cursor(&mut self) {
        self.draw_char(self.cursor_x_pos, self.cursor_y_pos, self.get(self.cursor_x_pos, self.cursor_y_pos));
    }
    
    fn draw_cursor(&mut self, x_pos: usize, y_pos: usize) {
        self.clear_cursor();
        let cursor = Letter {
            ascii_character: '■',
            color: self.font_color,
        };
        self.cursor_x_pos = x_pos + 1;
        self.cursor_y_pos = y_pos;


        self.draw_cursor_timer += 1;
        if self.draw_cursor_timer >= self.max_draw_cursor_timer {
            self.draw_cursor = !self.draw_cursor;
            self.draw_cursor_timer = 0;
        }

        if self.draw_cursor{ 
            self.draw_char(self.cursor_x_pos, self.cursor_y_pos, cursor);
        }
    }

    pub fn blink_cursor(&mut self) {
        self.draw_cursor(self.cursor_x_pos - 1, self.cursor_y_pos);
    }
}

use core::fmt;

impl fmt::Write for FontRenderer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.print_string(s);
        Ok(())
    }
}