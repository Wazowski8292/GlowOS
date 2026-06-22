#[derive(Copy, Clone)]
pub struct EventRingDequeuePointer(pub u64);

impl EventRingDequeuePointer {
    pub fn get_dequeue_segment_index(&self) -> u64{
        self.0 & 0x3
    }

    pub fn get_event_handler_busy(&self) -> u64 {
        (self.0 >> 3) & 0x1
    }

    pub fn get_event_ring_dequeue_pointer(&self) -> u64 {
        self.0 >> 4
    }

    pub fn set_dequeue_segment_index(&mut self, bit: u8) {
        self.0 = self.0 | bit as u64;
    }

    pub fn set_event_handler_busy(&mut self, bit: u8) {
        self.0 = self.0 | ((bit as u64) << 3);
    }

    pub fn set_event_ring_dequeue_pointer(&mut self, bit: u64) {
        self.0 = self.0 | (bit << 4);
    }
}

#[derive(Copy, Clone)]
pub struct XhciInterruptRegisters {
    pub interrupt_manager: u32,
    pub interrupt_moderation: u32,
    pub event_ring_segmentation_size: u32,
    pub reserved: u32,
    pub event_ring_segmentation_base_addres: u64,
    pub event_ring_dequeue_pointer: EventRingDequeuePointer,
}

pub struct XhciRuntimeRegister {
    pub microframe_index: u32,
    pub reserved: [u32; 7],
    pub interrupt_registers: [XhciInterruptRegisters; 1024],
}