use super::xhci_trb::XhciTransferRequestBlock;
use super::super::xhci::XhciDriver;

pub struct XhciCommandRing {
    max_trb_count: usize,
    enqueue_ptr: usize,
    trbs: *mut XhciTransferRequestBlock,
    pub physical_base: usize,
    pub ring_cycle_status: u8,
}

impl XhciCommandRing {
    pub fn new(max_trbs: usize, xhci_driver: &XhciDriver) -> Self{
        let ring_size = max_trbs * 16; // size_of(XhciTransferRequestBlock)
        
        #[allow(static_mut_refs)]
        let trbs = xhci_driver.alloc_memory(ring_size, 64) as *mut XhciTransferRequestBlock;
        let trbs_addr = xhci_driver.get_physical_addr(trbs as *mut u8) as usize;
        
        unsafe {
            let link = &mut *trbs.add(max_trbs - 1);
            link.parameter = trbs_addr as u64;
            link.control.set_trb_type(6);
            link.control.set_trb_link_bit();
            link.control.set_cycle(1);
        }

        Self {
            max_trb_count: max_trbs,
            ring_cycle_status: 1,
            enqueue_ptr: 0,
            trbs: trbs,
            physical_base: trbs_addr,
        }
    }

    pub fn enqueue(&mut self, trb: *mut XhciTransferRequestBlock) {
        unsafe{
            (*trb).control.set_cycle(self.ring_cycle_status);
            (*self.trbs.add(self.enqueue_ptr)) = (*trb).clone();
        }

        self.enqueue_ptr += 1;


        if self.enqueue_ptr == self.max_trb_count - 1 {
            unsafe {
                let link = &mut *self.trbs.add(self.max_trb_count - 1);
                link.control.set_trb_type(6);
                link.control.set_trb_link_bit();
                link.control.set_cycle(self.ring_cycle_status);
            }
            self.ring_cycle_status ^= 1;
            self.enqueue_ptr = 0;
        }
    }
}