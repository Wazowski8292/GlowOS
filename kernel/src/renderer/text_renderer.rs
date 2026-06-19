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
    cmd_buffer: Vec<String>,
    cmd_index: usize,
    buffer: Vec<Letter>,
    scale: usize,
    max_chars_x: usize,
    max_chars_y: usize,
    max_screen_chars_y: usize,
    x_pos: usize,
    y_pos: usize,
    scroll_y_pos: isize,
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
            cmd_buffer: Vec::new(),
            cmd_index: 0,
            buffer: vec![DEFAULT_LETTER; max_x * max_y],
            scale: scale,
            max_chars_x: max_x,
            max_chars_y: max_y,
            max_screen_chars_y: max_y,
            x_pos: 0,
            y_pos: 0,
            scroll_y_pos: 0,
            cursor_x_pos: 0,
            cursor_y_pos: 0,
            draw_cursor_timer: 0,
            max_draw_cursor_timer: 500,
            draw_cursor: true,
            background_color: bg_color,
            font_color: text_color,
        }
    }

    fn get(&self, col: usize, row: usize) -> Letter {
        if col < self.max_chars_x && row < self.max_chars_y{
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
            self.new_line();
        }
    }

    fn draw_char(&self, x_pos: usize, y_pos: usize, letter: Letter, reverse: bool) {
        let screen_y = y_pos as isize - self.scroll_y_pos;
        if screen_y < 0 || screen_y >= self.max_screen_chars_y as isize {
            return;
        }
        let screen_y = screen_y as usize;

        #[allow(static_mut_refs)]
        let renderer = unsafe { RENDERER.as_mut().unwrap() };
        let bitmap: u64 = SYS_FONT[char_to_font_index(letter.ascii_character).unwrap_or(0) as usize];

        for y in 0..CHAR_SIZE {
            for x in 0..CHAR_SIZE {
                let bit = y * CHAR_SIZE + x;
                let bit_set = (bitmap >> bit) & 1 == 1;
                let color = if bit_set != reverse {
                    letter.color
                } else {
                    self.background_color
                };

                for rel_x in 0..self.scale {
                    for rel_y in 0..self.scale {
                        renderer.put_pixel(
                            x_pos * CHAR_SIZE * self.scale + x * self.scale + rel_x,
                            screen_y * CHAR_SIZE * self.scale + y * self.scale + rel_y,
                            color);
                    }
                }
            }
        }
    }

    pub fn draw_buffer(&self)  {
        for screen_y in 0..self.max_screen_chars_y {
            let row_isize = screen_y as isize + self.scroll_y_pos;
            if row_isize >= 0 {
                let row = row_isize as usize;
                for col in 0..self.max_chars_x {
                    let letter = self.get(col, row);
                    self.draw_char(col, row, letter, false);
                }
            }
        }
    }

    fn new_line(&mut self) {
        self.y_pos += 1; 
        self.x_pos = 0; 
        if self.y_pos >= self.max_chars_y {
            for _ in 0..self.max_chars_x {
                self.buffer.push(DEFAULT_LETTER);
            }
            self.max_chars_y += 1;
        }
        
        let max_scroll = (self.buffer.len() / self.max_chars_x).saturating_sub(self.max_screen_chars_y) as isize;
        if self.scroll_y_pos < max_scroll {
            self.scroll_y_pos = max_scroll;
            self.draw_buffer();
        }
        self.draw_cursor(self.x_pos, self.y_pos);
    }

    fn print_char(&mut self, c: char) {
        let msg = Letter {
            ascii_character: c,
            color: self.font_color,
        };
        let old_x = self.x_pos;
        let old_y = self.y_pos;
        self.set(msg); // increments x_pos
        self.draw_char(old_x, old_y, msg, false); // draw glyph at where it was written
        self.draw_cursor(self.x_pos, self.y_pos); // draw cursor at new position
    }

    pub fn print_string(&mut self, msg: &str) {
        for letter in msg.chars() {
            match letter {
                '\n' => {self.new_line()},
                '\t' => {self.print_string("    ")},
                _ => {self.print_char(letter)},
            }
        }
    }

    pub fn backspace(&mut self) {

        self.x_pos -= if self.x_pos == 0 {
            0
        } else {
           1
        };

        self.buffer[self.x_pos + self.y_pos * self.max_chars_x] = DEFAULT_LETTER;
        self.draw_char(self.x_pos, self.y_pos, DEFAULT_LETTER, false);



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

    pub fn parse_last_line(&mut self) -> Vec<String>{
        let last_line = self.parse_line(self.y_pos - 1);
        if !last_line.is_empty() && last_line[0].chars().nth(0).unwrap_or(' ') == '$' {
            self.add_cmds(last_line.clone());
        }

        last_line
    }

    fn add_cmds(&mut self, cmd: Vec<String>) {
        if self.cmd_buffer.len() == 0 || self.cmd_buffer[self.cmd_buffer.len() - 1] != cmd.join(" ") {
            self.cmd_buffer.push(cmd.join(" "));
        }
    }

    fn clear_current_line(&mut self) {
        let row = self.y_pos;
        for col in 0..self.max_chars_x {
            self.buffer[row * self.max_chars_x + col] = DEFAULT_LETTER;
            self.draw_char(col, row, DEFAULT_LETTER, false);
        }
        self.x_pos = 0;
        self.draw_cursor(self.x_pos, self.y_pos);
    }

    fn write_cmd(&mut self) {
        if self.cmd_index >= self.cmd_buffer.len() {
            return;
        }
        self.clear_current_line();
        self.print_string(&self.cmd_buffer[self.cmd_index].clone());
    }

    pub fn history_older(&mut self) {
        if self.cmd_buffer.is_empty() {
            return;
        }
        if self.cmd_index > 0 {
            self.cmd_index -= 1;
        }
        self.write_cmd();
    }

    pub fn history_newer(&mut self) {
        if self.cmd_buffer.is_empty() {
            return;
        }
        if self.cmd_index + 1 < self.cmd_buffer.len() {
            self.cmd_index += 1;
            self.write_cmd();
        } else {
            self.cmd_index = self.cmd_buffer.len();
            self.clear_current_line();
        }
    }

    pub fn reset_idx_cmd(&mut self) {
        self.cmd_index = self.cmd_buffer.len();
    }

    pub fn clear_buffer(&mut self) {
        self.buffer.clear();
        self.buffer.resize(self.max_chars_x * self.max_screen_chars_y, DEFAULT_LETTER);
        self.max_chars_y = self.max_screen_chars_y;
        self.x_pos = 0;
        self.y_pos = 0;
        self.scroll_y_pos = 0;
        self.draw_cursor(self.x_pos, self.y_pos);
    }
    
    fn clear_cursor(&mut self) {
        self.draw_char(self.cursor_x_pos, self.cursor_y_pos, self.get(self.cursor_x_pos, self.cursor_y_pos), false);
    }
    
    fn draw_cursor(&mut self, x_pos: usize, y_pos: usize) {
        self.clear_cursor();
        self.cursor_x_pos = x_pos;
        self.cursor_y_pos = y_pos;

        if !self.draw_cursor {
            return;
        }
        let cursor = Letter {
            ascii_character: self.get(x_pos, y_pos).ascii_character,
            color: self.font_color,
        };
        self.draw_char(self.cursor_x_pos, self.cursor_y_pos, cursor, true);
    }

    /// Called from the timer IRQ — advances the blink state then redraws.
    pub fn blink_cursor(&mut self) {
        self.draw_cursor_timer += 1;
        if self.draw_cursor_timer >= self.max_draw_cursor_timer {
            self.draw_cursor = !self.draw_cursor;
            self.draw_cursor_timer = 0;
        }
        self.draw_cursor(self.cursor_x_pos, self.cursor_y_pos);
    }

    pub fn scroll_up(&mut self) {
        if self.scroll_y_pos > 0 {
            self.scroll_y_pos -= 1;
            self.draw_buffer();
        }
    }

    pub fn scroll_down(&mut self) {
        let max_scroll = (self.buffer.len() / self.max_chars_x).saturating_sub(self.max_screen_chars_y) as isize;
        if self.scroll_y_pos < max_scroll {
            self.scroll_y_pos += 1;
            self.draw_buffer();
        }
    }

    pub fn change_font_color(&mut self, color: Vec<String>) {
        if Color::from_str(&color[1]).is_err() {
            self.print_string("The font color doesn't match with any color");
            return;
        }
        self.font_color = Color::from_str(&color[1]).unwrap_or(Color::new(255, 255, 255));

        if color.len() == 1 || Color::from_str(&color[2]).is_err() {
            self.print_string("The font color doesn't match with any color");
            return;
        }
        #[allow(static_mut_refs)]
        let renderer = unsafe { RENDERER.as_mut().unwrap() };

        self.background_color = Color::from_str(&color[2]).unwrap_or(Color::new(0, 0, 0));
        renderer.change_background_color(Color::from_str(&color[2]).unwrap_or(Color::new(0, 0, 0)));

        self.draw_buffer();
    }

    pub fn update_color(&mut self) {
        for row in 0..self.max_chars_y {
            for col in 0..self.max_chars_x {
                let letter = Letter {
                    ascii_character: self.get(col, row).ascii_character,
                    color: self.font_color,
                };

                self.buffer[col + row * self.max_chars_x] = letter;
                self.draw_char(col, row, letter, false);
            }
        }
    }
}

use core::fmt;

impl fmt::Write for FontRenderer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.print_string(s);
        Ok(())
    }
}