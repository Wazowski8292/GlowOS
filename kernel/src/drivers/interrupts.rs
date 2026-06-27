use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

use crate::print;
use crate::println;
use crate::renderer::renderer::get_renderer;
use lazy_static::lazy_static;

use super::gdt;
use crate::hlt_loop;
use x86_64::structures::idt::PageFaultErrorCode;

pub static mut TIMER: usize = 0;
const HZ: u32 = 1000;

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.page_fault.set_handler_fn(page_fault_handler);
        unsafe {
            idt.double_fault
                .set_handler_fn(double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
        }
        idt[InterruptIndex::Timer.as_usize()].set_handler_fn(timer_interrupt_handler);
        idt[InterruptIndex::Keyboard.as_usize()].set_handler_fn(keyboard_interrupt_handler);
        idt[InterruptIndex::Xhci.as_usize()].set_handler_fn(xhci_interrupt_handler);
        idt
    };
}

fn init_pit(hz: u32) {
    use x86_64::instructions::port::Port;
    let divisor = (1_193_182u32 / hz) as u16;
    unsafe {
        let mut cmd: Port<u8> = Port::new(0x43);
        cmd.write(0x36);
        let mut data: Port<u8> = Port::new(0x40);
        data.write((divisor & 0xFF) as u8);
        data.write((divisor >> 8) as u8);
    }
}

pub fn init_idt() {
    IDT.load();

    unsafe { 
        let mut pics = PICS.lock();
        pics.initialize();
        pics.write_masks(0xF8, 0xFF);
    };

    init_pit(HZ);

    x86_64::instructions::interrupts::enable();
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    println!("EXEPTION: Break point detected!\n {:#?}", stack_frame);
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    panic!("EXEPTION: double fault\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    use x86_64::registers::control::Cr2;

    println!("EXCEPTION: Page fault");
    println!("Accessed Address: {:?}", Cr2::read());
    println!("Error Code: {:?}", error_code);
    println!("{:#?}", stack_frame);
    hlt_loop();
}

#[test_case]
fn test_breakpoint_exception() {
    x86_64::instructions::interrupts::int3();
}

use pic8259::ChainedPics;
use spin;

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

pub static PICS: spin::Mutex<ChainedPics> =
    spin::Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
    Keyboard,
    Xhci,
}

impl InterruptIndex {
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    pub fn as_usize(self) -> usize {
        usize::from(self.as_u8())
    }
}

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    get_renderer().font_renderer.blink_cursor();

    #[allow(static_mut_refs)]
    unsafe { 
        TIMER += 1;
    };

    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
    }
}

use crate::user::terminal;

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    use pc_keyboard::{DecodedKey, HandleControl, Keyboard, ScancodeSet1, layouts, KeyCode};
    use spin::Mutex;
    use x86_64::instructions::port::Port;

    lazy_static! {
        static ref KEYBOARD: Mutex<Keyboard<layouts::De105Key, ScancodeSet1>> =
            Mutex::new(Keyboard::new(
                ScancodeSet1::new(),
                layouts::De105Key,
                HandleControl::Ignore
            ));
    }

    let mut keyboard = KEYBOARD.lock();
    let mut port = Port::new(0x60);

    let scancode: u8 = unsafe { port.read() };

    if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
        if let Some(key) = keyboard.process_keyevent(key_event) {
            match key {
                DecodedKey::RawKey(KeyCode::PageDown) => {get_renderer().font_renderer.scroll_down()}
                DecodedKey::RawKey(KeyCode::ArrowDown) => {get_renderer().font_renderer.history_newer()}
                DecodedKey::RawKey(KeyCode::PageUp) => {get_renderer().font_renderer.scroll_up()}
                DecodedKey::RawKey(KeyCode::ArrowUp) => {get_renderer().font_renderer.history_older()}
                DecodedKey::Unicode('\n') => { println!(); terminal::command_runner(); }
                DecodedKey::Unicode('\x08') => { get_renderer().font_renderer.backspace(); }
                DecodedKey::Unicode(character) if character.is_ascii_graphic() || character == ' ' ||  character == '\t'=>
                {
                    print!("{}", character);
                    //last_line!(); Move cursor to last line
                }

                _ => {}
            }
        }
    }

    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
}

pub fn wait(time: usize) {
    #[allow(static_mut_refs)]
    let mut timer = unsafe { TIMER };

    while time / (HZ as usize) > timer {
    }
    timer = 0;
}

extern "x86-interrupt" fn xhci_interrupt_handler(_stack_frame: InterruptStackFrame) {
    unsafe {
        if let Some(driver) = &mut *(&raw mut crate::drivers::usb::xhci::XHCI_DRIVER) {
            driver.handle_irq();
        }

        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Xhci.as_u8());
    }
}