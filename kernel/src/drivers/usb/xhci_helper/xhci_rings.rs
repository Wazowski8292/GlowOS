use super::xhci_registers::{XhciInterruptRegisters, EventRingDequeuePointer};
use super::xhci_trb::XhciTransferRequestBlock;
use super::super::xhci::XhciDriver;
use crate::println;
use alloc::vec::Vec;
use alloc::vec;

#[derive(Copy, Clone)]
pub struct XhciCommandRing {
    max_trb_count: usize,
    enqueue_ptr: usize,
    trbs: *mut XhciTransferRequestBlock,
    pub physical_base: usize,
    pub ring_cycle_state: u8,
}

impl XhciCommandRing {
    pub fn new(max_trbs: usize, xhci_driver: &XhciDriver) -> Self{
        let ring_size = max_trbs * 16; // size_of(XhciTransferRequestBlock)
        
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
            ring_cycle_state: 1,
            enqueue_ptr: 0,
            trbs: trbs,
            physical_base: trbs_addr,
        }
    }

    pub fn enqueue(&mut self, trb: *mut XhciTransferRequestBlock) {
        unsafe{
            (*trb).control.set_cycle(self.ring_cycle_state);
            (*self.trbs.add(self.enqueue_ptr)) = (*trb).clone();
        }

        self.enqueue_ptr += 1;


        if self.enqueue_ptr == self.max_trb_count - 1 {
            unsafe {
                let link = &mut *self.trbs.add(self.max_trb_count - 1);
                link.control.set_trb_type(6);
                link.control.set_trb_link_bit();
                link.control.set_cycle(self.ring_cycle_state);
            }
            self.ring_cycle_state ^= 1;
            self.enqueue_ptr = 0;
        }
    }
}

struct XhciEventRingSegmentTableEntry {
    ring_segment_base_address: u64,
    ring_segment_size: u32,
    reserved: u32,
}

pub struct XhciEventRing {
    interrupter: *mut XhciInterruptRegisters,
    segment_trb_count: usize,
    segment_count: usize,
    trb: *mut XhciTransferRequestBlock,
    physical_base: usize,
    segment_table: *mut XhciEventRingSegmentTableEntry,
    dequeue_ptr: usize,
    ring_cycle_state: u8,
}

impl XhciEventRing {
    pub fn new(max_trbs: usize, interrupter: *mut XhciInterruptRegisters, xhci_driver: &XhciDriver) -> Self {
        let segment_size = max_trbs * 16;
        let segment_table_size = 16;

        let trbs = xhci_driver.alloc_memory(segment_size, 64) as *mut XhciTransferRequestBlock;
        let trbs_addr = xhci_driver.get_physical_addr(trbs as *mut u8) as u64;

        let segment_table = xhci_driver.alloc_memory(segment_table_size, 64) as *mut XhciEventRingSegmentTableEntry;
        let segment_table_addr = xhci_driver.get_physical_addr(segment_table as *mut u8) as u64;

        let entry = XhciEventRingSegmentTableEntry {
            ring_segment_base_address: trbs_addr,
            ring_segment_size: max_trbs as u32,
            reserved: 0,
        };
        unsafe { segment_table.write(entry) };

        let mut erdp = EventRingDequeuePointer(0);
        erdp.set_event_ring_dequeue_pointer(trbs_addr);
        erdp.set_event_handler_busy(1);
 
        unsafe {
            (*interrupter).event_ring_segmentation_size = 1;
            (*interrupter).event_ring_segmentation_base_addres = segment_table_addr;
            (*interrupter).event_ring_dequeue_pointer = erdp;
        }


        Self {
            interrupter,
            segment_trb_count: max_trbs,
            segment_count: 1,
            trb: trbs,
            physical_base: trbs_addr as usize,
            segment_table,
            dequeue_ptr: 0,
            ring_cycle_state: 1,
        }
    }

    fn update_event_ring_dequeue_pointer(&mut self) {
        unsafe {
            let mut erdp = EventRingDequeuePointer(0);
            erdp.set_event_ring_dequeue_pointer((self.physical_base + (self.dequeue_ptr * 16)) as u64);
            erdp.set_event_handler_busy(1); // write 1 to clear EHB, per spec
            (*self.interrupter).event_ring_dequeue_pointer = erdp;
        }
    }

    fn dequeue_ptr(&mut self) -> Option<*mut XhciTransferRequestBlock>{
        let trb = unsafe { (*self.trb.add(self.dequeue_ptr)).clone() };
        if trb.control.cycle_bit() != self.ring_cycle_state {
            println!("Event ring tryed to dequeue an invalid TRB");
            return None;
        }

        let trb = unsafe { &self.trb.add(self.dequeue_ptr) };
        self.dequeue_ptr += 1;

        if self.dequeue_ptr == self.segment_trb_count {
            self.dequeue_ptr = 0;
            self.ring_cycle_state ^= 1;
        } 

        Some(trb.clone())
    }

    pub fn has_unprocessed_events(&self) -> bool {
        unsafe { (*self.trb.add(self.dequeue_ptr)).control.cycle_bit() == self.ring_cycle_state }
    }

    pub fn dequeue_events(&mut self, trbs: &mut Vec<*const XhciTransferRequestBlock>) {
        while self.has_unprocessed_events() {
            if let Some(trb) = self.dequeue_ptr() {
                trbs.push(trb);
            } else {
                break;
            }
        }
 
        self.update_event_ring_dequeue_pointer();
    }
 
    fn flush_unprocessed_events(&mut self) {
        let mut events: Vec<*const XhciTransferRequestBlock> = vec![];
        self.dequeue_events(&mut events);
    }

}