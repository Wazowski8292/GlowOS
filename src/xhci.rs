use crate::memory;
use crate::pci;
use crate::println;
use x86_64::structures::paging::{Page, PhysFrame, Mapper, Size4KiB, Translate};
use x86_64::VirtAddr;
use crate::memory::MEMORY_MANAGER;
use volatile::Volatile;
use bootloader_api::BootInfo;

#[repr(C)]
struct AllocationHeader {
    page_count: usize,
}

#[repr(C)]
pub struct XhciCapabilityRegisters {
    pub caplength: Volatile<u8>,
    pub reserved: Volatile<u8>,
    pub hciversion: Volatile<u16>,
    pub hcsparams1: Volatile<u32>,
    pub hcsparams2: Volatile<u32>,
    pub hcsparams3: Volatile<u32>,
    pub hccparams1: Volatile<u32>,
    pub dboff: Volatile<u32>,
    pub rtssoff: Volatile<u32>,
    pub hccparams2: Volatile<u32>,
}

#[repr(C)]
pub struct XhciOperationalRegisters {
    pub usbcmd: Volatile<u32>,
    pub usbsts: Volatile<u32>,
    pub pagesize: Volatile<u32>,
    pub reserved1: [Volatile<u32>; 2],
    pub dnctrl: Volatile<u32>,
    pub crcr: Volatile<u64>,
    pub reserved2: [Volatile<u32>; 4],
    pub dcbaap: Volatile<u64>,
    pub config: Volatile<u32>,
}

pub struct XhciDriver {
    cap_regs: *const XhciCapabilityRegisters,
    op_regs: *const XhciOperationalRegisters,
    op_regs_offset: usize,

    max_device_slots: u8,
    max_interrupters: u16,
    max_ports: u8,
    isochronous_scheduling_threshold: u8,
    erst_max: u8,
    max_scratchpad_buffers: u16,
    
    bit64_addressing_capability: bool,
    bandwidth_negotiation_capability: bool,
    context_size_64_bytes: bool,
    port_power_control: bool,
    port_indicators: bool,
    light_reset_capability: bool,
    extended_capabilities_offset: u32,
}

pub static mut XHCI_DRIVER: Option<XhciDriver> = None;

impl XhciDriver {
    pub unsafe fn new(xhci_mmio_base: u64) -> Self {
        let cap_regs = xhci_mmio_base as *const XhciCapabilityRegisters;

        let caplength = (*cap_regs).caplength.read() as usize;
        let hcsparams1 = (*cap_regs).hcsparams1.read();
        let hcsparams2 = (*cap_regs).hcsparams2.read();
        let hccparams1 = (*cap_regs).hccparams1.read();

        let max_device_slots = (hcsparams1 & 0xFF) as u8;
        let max_interrupters = ((hcsparams1 >> 8) & 0x7FF) as u16;
        let max_ports = ((hcsparams1 >> 24) & 0xFF) as u8;

        let isochronous_scheduling_threshold = (hcsparams2 & 0xF) as u8;
        let erst_max = ((hcsparams2 >> 4) & 0xF) as u8;
        let max_scratchpad_buffers = (((hcsparams2 >> 21) & 0x1F) | ((hcsparams2 >> 16) & 0x3E0)) as u16;

        let bit64_addressing_capability = (hccparams1 & (1 << 0)) != 0;
        let bandwidth_negotiation_capability = (hccparams1 & (1 << 1)) != 0;
        let context_size_64_bytes = (hccparams1 & (1 << 2)) != 0;
        let port_power_control = (hccparams1 & (1 << 3)) != 0;
        let port_indicators = (hccparams1 & (1 << 4)) != 0;
        let light_reset_capability = (hccparams1 & (1 << 5)) != 0;
        let extended_capabilities_offset = ((hccparams1 >> 16) & 0xFFFF) * 4;

        let op_regs = (xhci_mmio_base + caplength as u64) as *const XhciOperationalRegisters;

        Self {
            cap_regs,
            op_regs,
            op_regs_offset: caplength,
            max_device_slots,
            max_interrupters,
            max_ports,
            isochronous_scheduling_threshold,
            erst_max,
            max_scratchpad_buffers,
            bit64_addressing_capability,
            bandwidth_negotiation_capability,
            context_size_64_bytes,
            port_power_control,
            port_indicators,
            light_reset_capability,
            extended_capabilities_offset,
        }
    }

    /// Class Method: Allocate coherent DMA memory block
    pub fn alloc_memory(&self, size: usize, alignment: usize) -> *mut u8 {
        if alignment == 0 {
            panic!("Attempted xhci DMA allocation with alignment 0!\n");
        }

        let header_size = core::mem::size_of::<AllocationHeader>();
        let total_needed_bytes = size + header_size + alignment;
        let page_count = (total_needed_bytes + 4095) / 4096;

        #[allow(static_mut_refs)]
        let manager = unsafe { 
            MEMORY_MANAGER.as_mut().expect("Memory Manager not initialized!") 
        };
        
        let base_vaddr = manager.next_free_dma_vaddr;

        // Allocate physical frames from our core global manager
        let target_frame = manager.dma_allocator.allocate_contiguous(page_count)
            .expect("Xhci physical memory allocation failed!\n");

        // Set up the page tables
        let target_page = Page::containing_address(base_vaddr);
        memory::map_xhci_contiguous_region(
            target_page,
            target_frame,
            page_count,
            &mut manager.mapper,
            &mut manager.dma_allocator,
        );

        manager.next_free_dma_vaddr = base_vaddr + (page_count * 4096) as u64;

        let raw_start = base_vaddr.as_u64();
        let user_vaddr_raw = (raw_start + header_size as u64 + (alignment as u64) - 1) & !((alignment as u64) - 1);
        let actual_header_addr = user_vaddr_raw - header_size as u64;

        unsafe {
            let header_ptr = actual_header_addr as *mut AllocationHeader;
            core::ptr::write(header_ptr, AllocationHeader { page_count });
        }

        let ptr = user_vaddr_raw as *mut u8;
        unsafe {
            core::ptr::write_bytes(ptr, 0, size);
        }

        ptr
    }

    pub fn free_memory(&self, ptr: *mut u8) {
        if ptr.is_null() {
            return;
        }

        let header_size = core::mem::size_of::<AllocationHeader>();
        let header_ptr = unsafe { ptr.offset(-(header_size as isize)) as *mut AllocationHeader };
        let header = unsafe { core::ptr::read(header_ptr) };

        #[allow(static_mut_refs)]
        let manager = unsafe { 
            MEMORY_MANAGER.as_mut().expect("Memory Manager not initialized!") 
        };

        let vaddr = VirtAddr::from_ptr(ptr);
        let start_page = Page::<Size4KiB>::containing_address(vaddr);
        let mut first_phys_frame: Option<PhysFrame> = None;

        for i in 0..header.page_count as u64 {
            let page = start_page + i;
            if let Some(phys_addr) = manager.mapper.translate_addr(page.start_address()) {
                let frame = PhysFrame::containing_address(phys_addr);
                if i == 0 {
                    first_phys_frame = Some(frame);
                }
                let (_unmap_result, flusher) = manager.mapper.unmap(page).unwrap();
                flusher.flush();
            }
        }

        if let Some(start_frame) = first_phys_frame {
            unsafe {
                manager.dma_allocator.deallocate_contiguous(start_frame, header.page_count);
            }
        }
    }

    pub fn get_physical_addr(&self, vaddr_ptr: *const u8) -> u64 {
        let vaddr = VirtAddr::from_ptr(vaddr_ptr);
        
        #[allow(static_mut_refs)]
        let manager = unsafe { 
            MEMORY_MANAGER.as_ref().expect("Memory Manager not initialized!") 
        };

        match manager.mapper.translate_addr(vaddr) {
            Some(phys_addr) => phys_addr.as_u64(),
            None => panic!("Attempted to look up unmapped xHCI physical address!"),
        }
    }
    pub fn log_capability_registers(&self) {
        println!("===== Xhci Capability Registers ({:p}) =====", self.cap_regs);
        println!("    Length                         : {}", self.op_regs_offset);
        println!("    Max Device Slots               : {}", self.max_device_slots);
        println!("    Max Interrupters               : {}", self.max_interrupters);
        println!("    Max Ports                      : {}", self.max_ports);
        println!("    IST                            : {}", self.isochronous_scheduling_threshold);
        println!("    ERST Max Size                  : {}", self.erst_max);
        println!("    Scratchpad Buffers             : {}", self.max_scratchpad_buffers);
        println!("    64-bit Addressing              : {}", if self.bit64_addressing_capability { "yes" } else { "no" });
        println!("    Bandwidth Negotiation          : {}", self.bandwidth_negotiation_capability);
        println!("    64-byte Context Size           : {}", if self.context_size_64_bytes { "yes" } else { "no" });
        println!("    Port Power Control             : {}", self.port_power_control);
        println!("    Port Indicators                : {}", self.port_indicators);
        println!("    Light Reset Available          : {}", self.light_reset_capability);
        println!();
    }
    pub fn log_operational_registers(&self) {
        unsafe {
            println!("===== Xhci Operational Registers ({:p}) =====", self.op_regs);
            println!("    usbcmd                         : {:#x}", (*self.op_regs).usbcmd.read());
            println!("    usbsts                         : {:#x}", (*self.op_regs).usbsts.read());
            println!("    pagesize                       : {:#x}", (*self.op_regs).pagesize.read());
            println!("    dnctrl                         : {:#x}", (*self.op_regs).dnctrl.read());
            println!("    crcr                           : {:#x}", (*self.op_regs).crcr.read());
            println!("    dcbaap                         : {:#x}", (*self.op_regs).dcbaap.read());
            println!("    config                         : {:#x}", (*self.op_regs).config.read());
            println!();
        }
    }
}
pub fn init(boot_info: &'static BootInfo){
    if let Some(xhci_phys_addr) = pci::init() {
        let xhci_base_vaddr = boot_info.physical_memory_offset.into_option().expect("Physical memory offset not found") + xhci_phys_addr;
        let xhci_driver = unsafe { XhciDriver::new(xhci_base_vaddr) };
        xhci_driver.log_capability_registers();
        xhci_driver.log_operational_registers();

        unsafe { XHCI_DRIVER =  Some(xhci_driver)};
    } else {
        println!("Error: No hardware xHCI controller detected on the PCI bus.");
    }
}