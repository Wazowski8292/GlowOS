use bootloader::UefiBoot;
use std::path::PathBuf;

fn main() {
    let kernel = PathBuf::from(std::env::args().nth(1).unwrap());
    
    if !kernel.exists() {
        eprintln!("Error: kernel not found at {:?}", kernel);
        eprintln!("Run `cargo build --package kernel` first");
        std::process::exit(1);
    }

    std::fs::create_dir_all("../target").unwrap();
    let out = PathBuf::from("../target/uefi.img");

    UefiBoot::new(&kernel).create_disk_image(&out).unwrap();
    println!("Created uefi.img");
}