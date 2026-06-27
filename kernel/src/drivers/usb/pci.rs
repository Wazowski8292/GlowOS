use crate::println;
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

fn pci_discover() -> Option<(u8, u8, u8, VirtAddr)> {
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
                            let vaddr = crate::memory::memory::map_mmio(paddr, 0x10000);
                            println!("xHCI BAR0 mapped:   {:#x}", vaddr.as_u64());
                            bios_handoff(vaddr.as_u64());
                            return Some((bus, dev, func, vaddr));
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

fn find_msi_capability(bus: u8, dev: u8, func: u8) -> Option<u8> {
    // Check if capabilities list is present (bit 4 of status register)
    let status = (pci_read_config(bus, dev, func, 0x04) >> 16) as u16;
    if (status & (1 << 4)) == 0 {
        println!("xHCI: no PCI capabilities list");
        return None;
    }

    // Start of capabilities list (offset 0x34, low byte)
    let mut cap_ptr = (pci_read_config(bus, dev, func, 0x34) & 0xFF) as u8;

    let mut safety = 0;
    while cap_ptr != 0 && safety < 32 {
        let cap_dword = pci_read_config(bus, dev, func, cap_ptr);
        let cap_id = (cap_dword & 0xFF) as u8;

        println!("  PCI cap at {:#x}: id={:#x}", cap_ptr, cap_id);

        if cap_id == 0x05 {
            println!("  Found MSI capability at offset {:#x}", cap_ptr);
            return Some(cap_ptr);
        }

        cap_ptr = ((cap_dword >> 8) & 0xFF) as u8;
        safety += 1;
    }

    println!("xHCI: MSI capability not found");
    None
}

pub fn enable_msi(bus: u8, dev: u8, func: u8, vector: u8) -> bool {
    let Some(msi_offset) = find_msi_capability(bus, dev, func) else {
        return false;
    };

    // MSI Message Control (bits 16-31 of cap dword 0)
    let cap0 = pci_read_config(bus, dev, func, msi_offset);
    let ctrl = ((cap0 >> 16) & 0xFFFF) as u16;
    let is_64bit = (ctrl & (1 << 7)) != 0;

    println!("MSI ctrl={:#x} 64bit={}", ctrl, is_64bit);

    // Message Address: 0xFEE000XX where XX encodes destination CPU
    // 0xFEE00000 = CPU 0, physical mode
    let msg_addr: u32 = 0xFEE00000;

    // Message Data: vector number, edge triggered, fixed delivery
    // Bits [7:0]  = vector
    // Bits [10:8] = delivery mode (000 = fixed)
    // Bit  [14]   = level (0 = edge)
    // Bit  [15]   = trigger mode (0 = edge)
    let msg_data: u16 = vector as u16;

    unsafe {
        // Write Message Address (offset + 4)
        pci_write_config(bus, dev, func, msi_offset + 4, msg_addr);

        if is_64bit {
            // Upper 32 bits of address (offset + 8) = 0 for low 4GB
            pci_write_config(bus, dev, func, msi_offset + 8, 0);
            // Message Data at offset + 12 for 64-bit MSI
            pci_write_config(bus, dev, func, msi_offset + 12, msg_data as u32);
        } else {
            // Message Data at offset + 8 for 32-bit MSI
            pci_write_config(bus, dev, func, msi_offset + 8, msg_data as u32);
        }
    }

    // Enable MSI (bit 0 of message control) and ensure only 1 vector (bits 4:1 = 0)
    let new_ctrl = (ctrl & !(0x7 << 4)) | 1;
    let new_cap0 = (cap0 & 0x0000_FFFF) | ((new_ctrl as u32) << 16);
    pci_write_config(bus, dev, func, msi_offset, new_cap0);

    println!("MSI enabled: vector={:#x} addr={:#x} data={:#x}", vector, msg_addr, msg_data);
    true
}

pub fn init() -> Option<(u8, u8, u8, VirtAddr)> {
    println!("PCI discovery start");
    pci_discover()
}