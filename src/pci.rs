use crate::println;
use x86_64::instructions::port::Port;
use core::ptr::{read_volatile, write_volatile};
use spin::Mutex;

const PCI_CONFIG_ADDRESS: u16 = 0xCF8;
const PCI_CONFIG_DATA: u16 = 0xCFC;


static BASE_ADDRESS: Mutex<u64> = Mutex::new(0);
static EXT_CAP_PTR: Mutex<u16> = Mutex::new(0);

fn pci_read_config(bus: u8, dev: u8, func: u8, offset: u8) -> u32 {
    let mut addr_port = Port::<u32>::new(PCI_CONFIG_ADDRESS);
    let mut data_port = Port::<u32>::new(PCI_CONFIG_DATA);

    let address = (1 << 31)

        | ((bus as u32) << 16)
        | ((dev as u32) << 11)
        | ((func as u32) << 8)

        | ((offset & 0xFC) as u32);

    unsafe{
        addr_port.write(address);
        data_port.read()
    }
}

fn pci_write_config(bus: u8, dev: u8, func: u8, offset: u8, value: u32) {
    let mut addr_port = Port::<u32>::new(PCI_CONFIG_ADDRESS);
    let mut data_port = Port::<u32>::new(PCI_CONFIG_DATA);

    let address = (1 << 31)
        | ((bus as u32) << 16)
        | ((dev as u32) << 11)

        | ((func as u32) << 8)
        | ((offset & 0xFC) as u32);
    
    unsafe {
        addr_port.write(address);
        data_port.write(value);
    }
}

fn enable_bus_mastering(bus: u8, dev: u8, func: u8) {
    let config_val = pci_read_config(bus, dev, func, 0x04);
    let mut command_reg = (config_val & 0xFFFF) as u16;

    command_reg |= 1 << 2;

    let updated_val = (config_val & 0xFFFF_0000) | (command_reg as u32);

    pci_write_config(bus, dev, func, 0x04, updated_val);
}

fn read_bar0(bus: u8, dev: u8, func: u8) -> u64 {
    let low = pci_read_config(bus, dev, func, 0x10);

    // 64-bit BAR check
    if (low & 0b110) == 0b100 {
        let high = pci_read_config(bus, dev, func, 0x14);

        ((high as u64) << 32) | ((low & 0xFFFF_FFF0) as u64)
    } else {
        (low & 0xFFFF_FFF0) as u64
    }
}

fn get_operational_base(base: u64) -> u64 {
    let cap_length = unsafe {
        *(base as *const u8)
    };

    base + cap_length as u64
}

fn save_extended_capabilities_pointer(mmio_base: usize) {
    let hccparams1 =
        unsafe { core::ptr::read_volatile((mmio_base + 0x10) as *const u32) };

    let xecp = ((hccparams1 >> 16) & 0xFFFF) as u16;

    if xecp == 0 {
        println!("No usable xHCI extended capabilities");
        return;
    }

    let mut ext_ptr = EXT_CAP_PTR.lock();
    *ext_ptr = xecp * 4;
}

fn get_xhci_controler(base: u64){

    let ext_cap_ptr = *EXT_CAP_PTR.lock(); 
    if ext_cap_ptr < 40 {
        println!("No usable xHCI extended capabilities");
        return;
    }

    let ext_cap_base =
        base + ((ext_cap_ptr as u64) * 4);

    // read USB Legacy Support capability
    let usblegsup =
        unsafe{read_volatile(ext_cap_base as *const u32)};

    let bios_owned =
        (usblegsup & (1 << 16)) != 0;

    if bios_owned {
        println!("BIOS owns xHCI controller");

        // set OS Owned Semaphore (bit 24)
        unsafe { write_volatile(
            ext_cap_base as *mut u32,
            usblegsup | (1 << 24),
        )};

        // wait for BIOS to release ownership
        loop {
            let val =
                unsafe {read_volatile(ext_cap_base as *const u32)};

            let bios_still_owned =
                (val & (1 << 16)) != 0;

            if !bios_still_owned {
                break;
            }
        }

        println!("OS now owns xHCI controller");
    }
}

fn pci_discover() {
    for bus in 0..=255 {
        for dev in 0..32 {
            for func in 0..8 {

                let vendor_device =
                    pci_read_config(bus, dev, func, 0x00) ;
                let vendor_id =
                    (vendor_device & 0xFFFF) as u16;

                // No device present
                if vendor_id == 0xFFFF {
                    continue;
                }

                let class_info =
                    pci_read_config(bus, dev, func, 0x08);

                let class =
                    ((class_info >> 24) & 0xFF) as u8;

                let subclass =
                    ((class_info >> 16) & 0xFF) as u8;

                let prog_if =
                    ((class_info >> 8) & 0xFF) as u8;

                // xHCI
                if class == 0x0C &&
                subclass == 0x03 &&
                prog_if == 0x30
                {
                    enable_bus_mastering(bus, dev, func);
                    let mut bar = BASE_ADDRESS.lock();
                    *bar = read_bar0(bus, dev, func);
                    let bar0 = pci_read_config(bus, dev, func, 0x10);

                    
                    //ave_extended_capabilities_pointer(bar0 as usize);

                    get_xhci_controler(*bar);
                    
                    println!("xHCI at bus {} dev {} func {}", bus, dev, func);
                    break;
                }
            }
        }
    }
}

pub fn init()  {
    pci_discover();
}