use x86_64::{
    PhysAddr, VirtAddr,
    structures::paging::{
        FrameAllocator, FrameDeallocator, Mapper, OffsetPageTable, Page, PageTable, PhysFrame, Size4KiB, PageTableFlags
    },
};
use bootloader_api::info::{MemoryRegions, MemoryRegionKind};

pub struct MemoryKernelManager {
    pub mapper: OffsetPageTable<'static>,
    pub dma_allocator: BitmapFrameAllocator,
    pub next_free_dma_vaddr: VirtAddr,
}

pub static mut MEMORY_MANAGER: Option<MemoryKernelManager> = None;

pub fn init(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    unsafe {
    let level_4_table = active_level_4_table(physical_memory_offset);
    OffsetPageTable::new(level_4_table, physical_memory_offset)
    }
}

unsafe fn active_level_4_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
    use x86_64::registers::control::Cr3;

    let (level_4_table_frame, _) = Cr3::read();

    let phys = level_4_table_frame.start_address();
    let virt = physical_memory_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    &mut *page_table_ptr
}

pub fn map_xhci_contiguous_region(
    start_page: Page,
    start_frame: PhysFrame,
    count: usize,
    mapper: &mut OffsetPageTable,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) {
    let flags = PageTableFlags::PRESENT 
              | PageTableFlags::WRITABLE 
              | PageTableFlags::NO_CACHE 
              | PageTableFlags::WRITE_THROUGH;

    for i in 0..count as u64 {
        let current_page = start_page + i;
        let current_frame = start_frame + i;

        let map_to_result = unsafe {
            mapper.map_to(current_page, current_frame, flags, frame_allocator)
        };
        
        match map_to_result {
            Ok(flusher) => flusher.flush(),
            Err(e) => panic!("Failed to map contiguous xHCI region at page {:?}: {:?}", current_page, e),
        }
    }
}

pub fn create_example_mapping(
    page: Page,
    frame: PhysFrame,
    mapper: &mut OffsetPageTable,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) {
    let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;

    let map_to_result = unsafe {
        mapper.map_to(page, frame, flags, frame_allocator)
    };
    map_to_result.expect("map_to failed").flush();
}

pub struct EmptyFrameAllocator;

unsafe impl FrameAllocator<Size4KiB> for EmptyFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        None
    }
}

pub struct BootInfoFrameAllocator {
    memory_map: &'static MemoryRegions,
    next: usize,
}

impl BootInfoFrameAllocator {
    pub unsafe fn init(memory_map: &'static MemoryRegions) -> Self {
        BootInfoFrameAllocator {
            memory_map,
            next: 0,
        }
    }

    fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> {
        self.memory_map
            .iter()
            .filter(|r| r.kind == MemoryRegionKind::Usable)
            .flat_map(|r| r.start..r.end)
            .step_by(4096)
            .map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
    }
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        let frame = self.usable_frames().nth(self.next);
        self.next += 1;
        frame
    }
}

pub struct BitmapFrameAllocator {
    bitmap: &'static mut [u8],
    total_frames: usize,
}

impl BitmapFrameAllocator {
    pub unsafe fn new(bitmap_storage: &'static mut [u8], total_frames: usize) -> Self {
        BitmapFrameAllocator {
            bitmap: bitmap_storage,
            total_frames,
        }
    }

    fn is_allocated(&self, frame_idx: usize) -> bool {
        let byte_idx = frame_idx / 8;
        let bit_idx = frame_idx % 8;
        (self.bitmap[byte_idx] & (1 << bit_idx)) != 0
    }

    fn set_range(&mut self, start_idx: usize, count: usize, allocated: bool) {
        for i in 0..count {
            let idx = start_idx + i;
            let byte_idx = idx / 8;
            let bit_idx = idx % 8;
            if allocated {
                self.bitmap[byte_idx] |= 1 << bit_idx;
            } else {
                self.bitmap[byte_idx] &= !(1 << bit_idx);
            }
        }
    }

    pub fn allocate_contiguous(&mut self, count: usize) -> Option<PhysFrame> {
        let mut continuous_found = 0;
        let mut start_idx = 0;

        for frame_idx in 0..self.total_frames {
            if !self.is_allocated(frame_idx) {
                if continuous_found == 0 {
                    start_idx = frame_idx;
                }
                continuous_found += 1;

                if continuous_found == count {
                    self.set_range(start_idx, count, true);
                    let phys_addr = PhysAddr::new((start_idx * 4096) as u64);
                    return Some(PhysFrame::containing_address(phys_addr));
                }
            } else {
                continuous_found = 0;
            }
        }
        None
    }

    pub unsafe fn deallocate_contiguous(&mut self, start_frame: PhysFrame, count: usize) {
        let start_idx = (start_frame.start_address().as_u64() / 4096) as usize;
        
        self.set_range(start_idx, count, false);
    }

    pub fn fill_from_boot_allocator(&mut self, boot_alloc: &BootInfoFrameAllocator) {
        self.bitmap.fill(0xFF);

        for frame in boot_alloc.usable_frames() {
            let frame_idx = (frame.start_address().as_u64() / 4096) as usize;
            if frame_idx < self.total_frames {
                let byte_idx = frame_idx / 8;
                let bit_idx = frame_idx % 8;
                self.bitmap[byte_idx] &= !(1 << bit_idx);
            }
        }

        for frame_idx in 0..boot_alloc.next {
            let byte_idx = frame_idx / 8;
            let bit_idx = frame_idx % 8;
            self.bitmap[byte_idx] |= 1 << bit_idx;
        }
    }
}

unsafe impl FrameAllocator<Size4KiB> for BitmapFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        self.allocate_contiguous(1)
    }
}

impl FrameDeallocator<Size4KiB> for BitmapFrameAllocator {
    unsafe fn deallocate_frame(&mut self, frame: PhysFrame) {
        let frame_idx = (frame.start_address().as_u64() / 4096) as usize;
        self.set_range(frame_idx, 1, false);
    }
}