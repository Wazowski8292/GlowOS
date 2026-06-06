use crate::renderer::{text_font, Color};

const MAX_CHARS_X: usize = 5;
const MAX_CHARS_Y: usize = 5;

struct Letter {
    ascii_character: u8,
    color: Color,
}

struct FontRenderer {
    buffer: [[Letter; MAX_CHARS_X]; MAX_CHARS_Y],
}