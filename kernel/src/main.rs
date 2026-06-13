#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(kernel::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;
use kernel::println;
use kernel::serial_println;
//use kernel::write_byte;


use bootloader_api::{BootInfo, config::Mapping, BootloaderConfig};

pub static BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(Mapping::Dynamic);
    config
};
bootloader_api::entry_point!(kernel_main, config = &BOOTLOADER_CONFIG);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    kernel::init(boot_info);
    

    /*print_logo();

    println!("\n============ Init when correctly ===========\n");

    println!("Hello user!");
    println!("You can type $help to get the list of commands");

    #[cfg(test)]
    test_main();
    */
    kernel::hlt_loop();
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
            //write_byte!(logo_rows[row][col]); 
        }
    }
    println!("\n\n");
}

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    serial_println!("{}", info);
    kernel::hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    kernel::test_panic_handler(info)
}

