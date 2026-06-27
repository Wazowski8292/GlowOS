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
        self.0 & !0xF
    }

    pub fn set_dequeue_segment_index(&mut self, bit: u8) {
        self.0 = self.0 | bit as u64;
    }

    pub fn set_event_handler_busy(&mut self, bit: u8) {
        self.0 = self.0 | ((bit as u64) << 3);
    }

    pub fn set_event_ring_dequeue_pointer(&mut self, ptr: u64) {
        self.0 = (self.0 & 0xF) | (ptr & !0xF);
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct XhciInterruptRegisters {
    pub interrupt_manager: u32,
    pub interrupt_moderation: u32,
    pub event_ring_segmentation_size: u32,
    pub reserved: u32,
    pub event_ring_segmentation_base_addres: u64,
    pub event_ring_dequeue_pointer: EventRingDequeuePointer,
}

#[repr(C)]
pub struct XhciRuntimeRegister {
    pub microframe_index: u32,
    pub reserved: [u32; 7],
    pub interrupt_registers: [XhciInterruptRegisters; 1024],
}

#[repr(C)]
pub struct XhciDoorbellRegister (u32);

impl XhciDoorbellRegister {
    fn get_doorbell_target(&self) -> u8 {
        (self.0 & 0xff) as u8
    }

    fn get_reserved(&self) -> u8 {
        ((self.0 >> 8) & 0xff) as u8
    }
    
    fn get_doorbell_id(&self) -> u16 {
        ((self.0 >> 16) & 0xffff) as u16
    }
    
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct XhciDoorbellManager {
    doorbell: *mut XhciDoorbellRegister,
}

impl XhciDoorbellManager {
    pub fn new(manager: *mut XhciDoorbellRegister) -> Self {
        Self{ 
            doorbell: manager
        }
    }

    fn ring_doorbell(&mut self, doorbell: u8, target: u8) {
        unsafe {
            self.doorbell
                .add(doorbell as usize)
                .write_volatile(XhciDoorbellRegister(target as u32));
        }
    }

    pub fn ring_command_doorbell(&mut self) {
        self.ring_doorbell(0, 0);
    }

    pub fn ring_control_endpoint_doorbell(&mut self, doorbell: u8) {
        self.ring_doorbell(doorbell, 1);
    }
 }