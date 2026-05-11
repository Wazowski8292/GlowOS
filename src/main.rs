#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(os::test_runner)]
#![reexport_test_harness_main = "test_main"]

use bootloader::{BootInfo, entry_point};
use core::panic::PanicInfo;
use os::println;
extern crate alloc;
use alloc::{boxed::Box, vec, vec::Vec, rc::Rc};
use os::allocator;

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static BootInfo) -> ! {

    println!("Hello World{}", "!");
    os::init(boot_info);

    #[cfg(test)]
    test_main();

    println!("It did not crash!");
    os::hlt_loop();
}

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    os::hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    os::test_panic_handler(info)
}
