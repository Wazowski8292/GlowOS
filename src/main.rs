/*
TODO:
    Merge linked list,
    Add more commands to terminal
    Scroll up and down -> No clear line when chars reach it
    Add a history of commands
    Add a way to insert letter in the middles of words without erraizing them

*/
#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(os::test_runner)]
#![reexport_test_harness_main = "test_main"]

use bootloader::{BootInfo, entry_point};
use core::panic::PanicInfo;
use os::println;
use os::write_byte;

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    print_logo();

    os::init(boot_info);

    println!("\n============ Init when correctly ===========\n");

    println!("Hello user!");
    println!("You can type $help to get the list of commands");

    #[cfg(test)]
    test_main();

    os::hlt_loop();
}

pub fn print_logo() {
    let logo_rows: [&[u8]; 10] = [
        b"\x20\x20\x20\x20\xC9\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xBB\n",
        b"\x20\x20\xC9\xCD\xBC\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\xC8\xCD\xBB\n",
        b"\x20\xC9\xBC\x20\x20\xC9\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xBB\x20\x20\xC8\xBB\n",
        b"\x20\xBA\x20\x20\x20\xBA\x20\xC9\xCD\xCD\xCD\xCD\xCD\xBB\x20\xBA\x20\x20\x20\xBA\n",
        b"\x20\xBA\x20\x20\x20\xBA\x20\xBA\x20\x20\x20\x20\x20\xBA\x20\xBA\x20\x20\x20\xBA\n",
        b"\x20\xC8\xBB\x20\x20\xBA\x20\xC8\xBB\x20\x20\x20\xC9\xBC\x20\xBA\x20\x20\xC9\xBC\n",
        b"\x20\x20\xC8\xBB\x20\xC8\xBB\x20\xC8\xBB\x20\xC9\xBC\x20\xC9\xBC\x20\xC9\xBC\n",
        b"\x20\x20\x20\xC8\xBB\x20\xC8\xBB\x20\xC8\xCD\xBC\x20\xC9\xBC\x20\xC9\xBC\n",
        b"\x20\x20\x20\x20\xC8\xBB\x20\xC8\xBB\x20\x20\x20\xC9\xBC\x20\xC9\xBC\n",
        b"\x20\x20\x20\x20\x20\xC8\xCD\xCD\xBC\x20\x20\x20\xC8\xCD\xCD\xBC\n"
    ];

    for row in 0..logo_rows.len() {
        for col in 0..logo_rows[row].len() {
            write_byte!(logo_rows[row][col]); 
        }
    }
    println!("\n\n");
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

