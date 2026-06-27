#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Control(pub u32);

impl Control {
    pub const fn new() -> Self { 
        Self(0)
    }
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

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct XhciCommandCompletionTrb {
    pub command_trb_pointer: u64,
    pub status: u32,
    pub control: u32,
}

impl XhciCommandCompletionTrb {
    pub fn completion_code(&self) -> u8 {
        ((self.status >> 24) & 0xFF) as u8
    }

    pub fn slot_id(&self) -> u8 {
        ((self.control >> 24) & 0xFF) as u8
    }

    pub fn vfid(&self) -> u8 {
        ((self.control >> 16) & 0xFF) as u8
    }

    pub fn trb_type(&self) -> u8 {
        ((self.control >> 10) & 0x3F) as u8
    }

    pub fn cycle_bit(&self) -> u8 {
        (self.control & 0x1) as u8
    }

    pub unsafe fn from_raw(trb: *const XhciTransferRequestBlock) -> Option<&'static Self> {
        let raw = &*trb;
        let trb_type = (raw.control.0 >> 10) & 0x3F;
        if trb_type == 33 {
            Some(&*(trb as *const XhciCommandCompletionTrb))
        } else {
            None
        }
    }
}

#[repr(u8)]
#[derive(Debug, PartialEq)]
pub enum CompletionCode {
    Invalid                 = 0,
    Success                 = 1,
    DataBuffer              = 2,
    BabbleDetected          = 3,
    UsbTransaction          = 4,
    Trb                     = 5,
    Stall                   = 6,
    ResourceError           = 7,
    Bandwidth               = 8,
    NoSlotsAvailable        = 9,
    InvalidStreamType       = 10,
    SlotNotEnabled          = 11,
    EndpointNotEnabled      = 12,
    ShortPacket             = 13,
    RingUnderrun            = 14,
    RingOverrun             = 15,
    VfEventRingFull         = 16,
    ParameterError          = 17,
    BandwidthOverrun        = 18,
    ContextStateError       = 19,
    NoPingResponse          = 20,
    EventRingFull           = 21,
    IncompatibleDevice      = 22,
    MissedService           = 23,
    CommandRingStopped      = 24,
    CommandAborted          = 25,
    Stopped                 = 26,
    StoppedLengthInvalid    = 27,
    MaxExitLatencyTooLarge  = 29,
    IsochBuffer             = 31,
    EventLost               = 32,
    Undefined               = 33,
    InvalidStreamId         = 34,
    SecondaryBandwidth      = 35,
    SplitTransaction        = 36,
}

impl CompletionCode {
    pub fn from_u8(val: u8) -> Self {
        match val {
            1  => Self::Success,
            2  => Self::DataBuffer,
            5  => Self::Trb,
            6  => Self::Stall,
            7  => Self::ResourceError,
            9  => Self::NoSlotsAvailable,
            13 => Self::ShortPacket,
            21 => Self::EventRingFull,
            24 => Self::CommandRingStopped,
            25 => Self::CommandAborted,
            _  => Self::Undefined,
        }
    }
}