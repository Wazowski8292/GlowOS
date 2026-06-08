use bootloader_api::info::{FrameBufferInfo, PixelFormat, FrameBuffer};
use text_renderer::FontRenderer;

pub mod text_renderer;
pub mod text_font;

pub static mut RENDERER: Option<Rernderer> = None; 

pub struct Rernderer {
    info: FrameBufferInfo,
    buffer: &'static mut [u8],
    background_color: Color,
    font_renderer: FontRenderer,
}

#[derive(Clone, Copy)]
pub struct Color {
    r: u8,
    g: u8,
    b: u8,
}

impl Color {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Color { r, g, b }
    }
}

impl Rernderer {
    pub fn new(framebuffer: &'static mut FrameBuffer, background_color: Color) -> Self{
        let fb = framebuffer;
        let info = fb.info();
        let buffer = fb.buffer_mut();
        let font_renderer = FontRenderer::new(info.width, info.height);

        Self {
            info: info,
            buffer: buffer,
            background_color: background_color,
            font_renderer: font_renderer,
        }
    }

    pub fn put_pixel(&mut self ,x: usize, y: usize, color: Color) {
        if x >= self.info.width || y >= self.info.height {
            return;
        }

        let offset = (x + y * self.info.stride) * self.info.bytes_per_pixel;
        match self.info.pixel_format {
            PixelFormat::Rgb => {
                self.buffer[offset]     = color.r;
                self.buffer[offset + 1] = color.g;
                self.buffer[offset + 2] = color.b;
            },
            PixelFormat::Bgr => {
                self.buffer[offset]     = color.b;
                self.buffer[offset + 1] = color.g;
                self.buffer[offset + 2] = color.r;
            },
            PixelFormat::U8 => {
                // Approximate luminance: 0.299R + 0.587G + 0.114B
                let luma = (color.r as u16 * 299
                    + color.g as u16 * 587
                    + color.b as u16 * 114)
                    / 1000;
                self.buffer[offset] = luma as u8;
            },
            PixelFormat::Unknown { red_position, green_position, blue_position } => {
                // Each position is the byte offset of that channel within the pixel slot.
                let bpp = self.info.bytes_per_pixel;
                if (red_position as usize) < bpp {
                    self.buffer[offset + red_position as usize]   = color.r;
                }
                if (green_position as usize) < bpp {
                    self.buffer[offset + green_position as usize] = color.g;
                }
                if (blue_position as usize) < bpp {
                    self.buffer[offset + blue_position as usize]  = color.b;
                }
            },
            _ => {} // future-proof against any new variants added upstream
        }
    }

    pub fn clear_screen(&mut self){
        for x in 0..self.info.width {
            for y in 0..self.info.height {
                self.put_pixel(x, y, self.background_color.clone());
            }
        }
    }
}

pub fn init(framebuffer: &'static mut FrameBuffer){
    let bg_color = Color::new(0,0,0);

    unsafe {RENDERER = Some(Rernderer::new(framebuffer, bg_color))}; 

    #[allow(static_mut_refs)]
    let renderer = unsafe{RENDERER.as_mut().unwrap()};
    renderer.clear_screen();
    renderer.font_renderer.test();
    renderer.font_renderer.draw_buffer();
}