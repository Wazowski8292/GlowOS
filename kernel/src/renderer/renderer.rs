#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::renderer::renderer::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    x86_64::instructions::interrupts::without_interrupts(|| {
        #[allow(static_mut_refs)]
        let renderer = unsafe { RENDERER.as_mut().unwrap() };
        renderer.font_renderer.write_fmt(args).ok();
    });
}

pub fn get_renderer() -> &'static mut Renderer {
    unsafe { 
        #[allow(static_mut_refs)]
        RENDERER.as_mut().unwrap() 
    }
}

use bootloader_api::info::{FrameBufferInfo, PixelFormat, FrameBuffer};
use super::font_renderer::text_renderer::FontRenderer;
use core::fmt;



pub static mut RENDERER: Option<Renderer> = None; 

pub struct Renderer {
    info: FrameBufferInfo,
    buffer: &'static mut [u8],
    background_color: Color,
    pub font_renderer: FontRenderer,
}

#[derive(Clone, Copy)]
pub struct Color {
    r: u8,
    g: u8,
    b: u8,
}

impl Color {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Color { r, g, b }
    }
    
    pub fn from_str(value: &str) -> Result<Self, ()> {
        match value {
            "black"     => Ok(Color::new(0,   0,   0  )),
            "blue"      => Ok(Color::new(0,   0,   170)),
            "green"     => Ok(Color::new(0,   170, 0  )),
            "cyan"      => Ok(Color::new(0,   170, 170)),
            "red"       => Ok(Color::new(170, 0,   0  )),
            "magenta"   => Ok(Color::new(170, 0,   170)),
            "brown"     => Ok(Color::new(170, 85,  0  )),
            "lightGray" => Ok(Color::new(170, 170, 170)),
            "darkGray"  => Ok(Color::new(85,  85,  85 )),
            "lightBlue" => Ok(Color::new(85,  85,  255)),
            "lightGreen"=> Ok(Color::new(85,  255, 85 )),
            "lightCyan" => Ok(Color::new(85,  255, 255)),
            "lightRed"  => Ok(Color::new(255, 85,  85 )),
            "pink"      => Ok(Color::new(255, 85,  255)),
            "yellow"    => Ok(Color::new(255, 255, 85 )),
            "white"     => Ok(Color::new(255, 255, 255)),
            _ => Err(()),
        }
    }
}

impl Renderer {
    pub fn new(framebuffer: &'static mut FrameBuffer, background_color: Color) -> Self{
        let fb = framebuffer;
        let info = fb.info();
        let buffer = fb.buffer_mut();
        let font_renderer = FontRenderer::new(info.width, info.height, background_color);

        Self {
            info: info,
            buffer: buffer,
            background_color: background_color,
            font_renderer: font_renderer,
        }
    }

    #[inline]
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

    #[inline]
    pub fn put_pixel_unchecked(&mut self ,x: usize, y: usize, color: Color) {
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
        for y in 0..self.info.height {
            for x in 0..self.info.width {
                self.put_pixel_unchecked(x, y, self.background_color);
            }
        }
    }

    pub fn change_background_color(&mut self, color: Color) {
        self.background_color = color;
    }
}

pub fn init(framebuffer: &'static mut FrameBuffer){
    let bg_color = Color::new(0,0,0);

    unsafe {RENDERER = Some(Renderer::new(framebuffer, bg_color))}; 

    #[allow(static_mut_refs)]
    let renderer = unsafe{RENDERER.as_mut().unwrap()};
    renderer.clear_screen();
}