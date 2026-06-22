#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Control(pub u32);

impl Control {
    pub const fn cycle_bit(&self) -> u8 { (self.0 & 1) as u8}
    pub const fn eval_next_trb(&self) -> bool { ((self.0 >> 1) & 1) != 0 }
    pub const fn interrupt_on_short_pkt(&self) -> bool { ((self.0 >> 2) & 1) != 0 }
    pub const fn no_snoop(&self) -> bool { ((self.0 >> 3) & 1) != 0 }
    pub const fn chain_bit(&self) -> bool { ((self.0 >> 4) & 1) != 0 }
    pub const fn interrupt_on_completion(&self) -> bool { ((self.0 >> 5) & 1) != 0 }
    pub const fn immediate_data(&self) -> bool { ((self.0 >> 6) & 1) != 0 }
    
    pub const fn rsvd0(&self) -> u32 { (self.0 >> 7) & 0x3 }
    
    pub const fn block_event_interrupt(&self) -> bool { ((self.0 >> 9) & 1) != 0 }

    pub const fn trb_type(&self) -> u32 {
        (self.0 >> 10) & 0x3F
    }

    pub fn set_trb_type(&mut self, value: u32) {
        let mask = 0x3F << 10;
        self.0 = (self.0 & !mask) | ((value & 0x3F) << 10);
    }

    pub const fn rsvd1(&self) -> u32 {
        (self.0 >> 16) & 0xFFFF
    }

    pub fn set_rsvd1(&mut self, value: u32) {
        let mask = 0xFFFF << 16;
        self.0 = (self.0 & !mask) | ((value & 0xFFFF) << 16);
    }

    pub fn set_trb_link_bit(&mut self) {
        self.0 |= 1 << 1;
    }

    pub fn set_cycle(&mut self, bit: u8) {
        self.0 |= bit as u32;
    }
}

#[repr(C)]
#[derive(Clone)]
pub struct XhciTransferRequestBlock {
    pub parameter: u64,
    pub status: u32,
    pub control: Control,
}