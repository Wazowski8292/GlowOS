#![no_std]
#![cfg_attr(test, no_main)]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]
#![feature(abi_x86_interrupt)]

pub mod drivers;
pub mod memory;
pub mod renderer;
pub mod user;

use core::panic::PanicInfo;

pub trait Testable {
    fn run(&self) -> ();
}

impl<T> Testable for T
where
    T: Fn(),
{
    fn run(&self) {
        serial_print!("{}...\t", core::any::type_name::<T>());
        self();
        serial_println!("[ok]");
    }
}

pub fn test_runner(tests: &[&dyn Testable]) {
    serial_println!("Running {} tests", tests.len());
    for test in tests {
        test.run();
    }
    exit_qemu(QemuExitCode::Success);
}

pub fn test_panic_handler(info: &PanicInfo) -> ! {
    serial_println!("[failed]\n");
    serial_println!("Error: {}\n", info);
    exit_qemu(QemuExitCode::Failed);
    hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    test_panic_handler(info)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11,
}

pub fn exit_qemu(exit_code: QemuExitCode) {
    use x86_64::instructions::port::Port;

    unsafe {
        let mut port = Port::new(0xf4);
        port.write(exit_code as u32);
    }
}

use bootloader_api::BootInfo;
use bootloader_api::info::FrameBuffer;

pub fn init(boot_info: &'static mut BootInfo) {
    let framebuffer = boot_info.framebuffer.as_mut().unwrap() as *mut FrameBuffer;
    let xhci_base_vaddr = boot_info.physical_memory_offset.into_option().expect("Physical memory offset not found");

    enable_local_apic();

    drivers::gdt::init();
    memory::allocator::alloc_init(boot_info);
    
    unsafe { renderer::renderer::init(&mut *framebuffer) };
    drivers::interrupts::init_idt();

    drivers::usb::xhci::init(xhci_base_vaddr);
}

pub fn enable_local_apic() {
    use x86_64::registers::model_specific::Msr;
    unsafe {
        let mut apic_base = Msr::new(0x1B).read();
        apic_base |= 1 << 11;
        Msr::new(0x1B).write(apic_base);
    }
}

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

#[cfg(test)]
use bootloader_api::{entry_point};

#[cfg(test)]
bootloader_api::entry_point!(test_kernel_main);

#[cfg(test)]
fn test_kernel_main(boot_info: &'static mut BootInfo) -> ! {
    init(boot_info);
    test_main();
    hlt_loop();
}

extern crate alloc;