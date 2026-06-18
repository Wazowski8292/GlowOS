use crate::{memory, println};
use x86_64::{VirtAddr, instructions::port::Port};
use core::ptr::{read_volatile, write_volatile};

const PCI_CONFIG_ADDRESS: u16 = 0xCF8;
const PCI_CONFIG_DATA: u16 = 0xCFC;

fn io_delay() {
    // Port 0x80 is the POST code port — safe to read on all real hardware
    let mut delay_port = Port::<u8>::new(0x80);
    unsafe { let _ = delay_port.read(); }
}

fn pci_read_config(bus: u8, dev: u8, func: u8, offset: u8) -> u32 {
    let mut addr_port = Port::<u32>::new(PCI_CONFIG_ADDRESS);
    let mut data_port = Port::<u32>::new(PCI_CONFIG_DATA);
    let address = (1u32 << 31)
        | ((bus as u32) << 16)
        | ((dev as u32) << 11)
        | ((func as u32) << 8)
        | ((offset & 0xFC) as u32);
    unsafe {
        addr_port.write(address);
        io_delay();
        data_port.read()
    }
}

fn pci_write_config(bus: u8, dev: u8, func: u8, offset: u8, value: u32) {
    let mut addr_port = Port::<u32>::new(PCI_CONFIG_ADDRESS);
    let mut data_port = Port::<u32>::new(PCI_CONFIG_DATA);
    let address = (1u32 << 31)
        | ((bus as u32) << 16)
        | ((dev as u32) << 11)
        | ((func as u32) << 8)
        | ((offset & 0xFC) as u32);
    unsafe {
        addr_port.write(address);
        io_delay();
        data_port.write(value);
    }
}

fn is_multifunction(bus: u8, dev: u8) -> bool {
    let header = pci_read_config(bus, dev, 0, 0x0C);
    let header_type = ((header >> 16) & 0xFF) as u8;
    (header_type & 0x80) != 0
}

fn enable_bus_mastering(bus: u8, dev: u8, func: u8) {
    let config_val = pci_read_config(bus, dev, func, 0x04);
    let mut command_reg = (config_val & 0xFFFF) as u16;
    // Enable bus mastering (bit 2) and memory space (bit 1)
    command_reg |= (1 << 2) | (1 << 1);
    let updated_val = (config_val & 0xFFFF_0000) | (command_reg as u32);
    pci_write_config(bus, dev, func, 0x04, updated_val);
    io_delay();
}

fn read_bar0(bus: u8, dev: u8, func: u8) -> Option<u64> {
    let low = pci_read_config(bus, dev, func, 0x10);

    // BAR must not be zero or all-ones (device not present / not implemented)
    if low == 0 || low == 0xFFFF_FFFF {
        return None;
    }

    // Must be a memory BAR (bit 0 = 0), not I/O BAR
    if (low & 0x1) != 0 {
        println!("xHCI BAR0 is I/O BAR, unexpected");
        return None;
    }

    let bar_type = (low >> 1) & 0x3;

    let base = if bar_type == 0b10 {
        // 64-bit BAR
        let high = pci_read_config(bus, dev, func, 0x14);
        ((high as u64) << 32) | ((low & 0xFFFF_FFF0) as u64)
    } else {
        // 32-bit BAR
        (low & 0xFFFF_FFF0) as u64
    };

    // Sanity check: address must be non-zero and within reasonable range
    if base == 0 || base > 0x0000_FFFF_FFFF_F000 {
        println!("xHCI BAR0 value looks invalid: {:#x}", base);
        return None;
    }

    Some(base)
}

fn pci_discover() -> Option<VirtAddr> {
    for bus in 0u8..=255 {
        for dev in 0u8..32 {
            let vendor_device = pci_read_config(bus, dev, 0, 0x00);
            let vendor_id = (vendor_device & 0xFFFF) as u16;
            if vendor_id == 0xFFFF {
                continue;
            }

            let multifunction = is_multifunction(bus, dev);

            for func in 0u8..8 {
                if func != 0 && !multifunction {
                    break;
                }

                let vendor_device = pci_read_config(bus, dev, func, 0x00);
                let vendor_id = (vendor_device & 0xFFFF) as u16;
                if vendor_id == 0xFFFF {
                    continue;
                }

                let class_info = pci_read_config(bus, dev, func, 0x08);
                let class    = ((class_info >> 24) & 0xFF) as u8;
                let subclass = ((class_info >> 16) & 0xFF) as u8;
                let prog_if  = ((class_info >>  8) & 0xFF) as u8;

                if class == 0x0C && subclass == 0x03 && prog_if == 0x30 {
                    println!("Found xHCI at bus={} dev={} func={}", bus, dev, func);
                    enable_bus_mastering(bus, dev, func);

                    match read_bar0(bus, dev, func) {
                        Some(paddr) => {
                            println!("xHCI BAR0 physical: {:#x}", paddr);
                            // Map the MMIO region into virtual address space FIRST,
                            // so bios_handoff never touches unmapped memory.
                            let vaddr = memory::map_mmio(paddr, 0x10000);
                            println!("xHCI BAR0 mapped:   {:#x}", vaddr.as_u64());
                            bios_handoff(vaddr.as_u64());
                            return Some(vaddr);
                        }
                        None => {
                            println!("xHCI BAR0 invalid, skipping");
                        }
                    }
                }
            }
        }
    }
    println!("No xHCI controller found");
    None
}

fn bios_handoff(vbase: u64) {
    println!("xHCI bios_handoff: virt={:#x}", vbase);

    let hccparams1 = unsafe { read_volatile((vbase + 0x10) as *const u32) };
    let ext_cap_ptr = ((hccparams1 >> 16) & 0xFFFF) as u32;

    if ext_cap_ptr < 40 {
        println!("No xHCI extended capabilities (ptr={:#x})", ext_cap_ptr);
        return;
    }

    let ext_cap_base = vbase + ((ext_cap_ptr as u64) * 4);
    let usblegsup = unsafe { read_volatile(ext_cap_base as *const u32) };

    let cap_id = (usblegsup & 0xFF) as u8;
    if cap_id != 0x01 {
        println!("First xHCI ext cap not USB Legacy Support (id={:#x})", cap_id);
        return;
    }

    let bios_owned = (usblegsup & (1 << 16)) != 0;
    if bios_owned {
        println!("BIOS owns xHCI, taking ownership...");
        unsafe { write_volatile(ext_cap_base as *mut u32, usblegsup | (1 << 24)) };

        let mut timeout = 1_000_000u32;
        loop {
            let val = unsafe { read_volatile(ext_cap_base as *const u32) };
            if (val & (1 << 16)) == 0 { break; }
            timeout -= 1;
            if timeout == 0 {
                println!("Timeout: BIOS did not release xHCI");
                return;
            }
        }
        println!("OS now owns xHCI controller");
    } else {
        println!("OS already owns xHCI controller");
    }
}

pub fn init() -> Option<VirtAddr> {
    println!("PCI discovery start");
    pci_discover()
}