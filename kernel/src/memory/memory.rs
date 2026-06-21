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
    pub next_free_mmio_vaddr: VirtAddr,
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

    unsafe{ &mut *page_table_ptr }
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

pub fn map_mmio(phys_base: u64, size: usize) -> VirtAddr {
    assert!(size > 0, "map_mmio: size must be > 0");

    let page_offset  = phys_base & 0xFFF;
    let phys_aligned = phys_base - page_offset;
    let total_bytes  = (size as u64 + page_offset + 0xFFF) & !0xFFF;
    let page_count   = (total_bytes / 4096) as usize;

    #[allow(static_mut_refs)]
    let manager = unsafe {
        MEMORY_MANAGER.as_mut().expect("map_mmio: MemoryKernelManager not initialised")
    };

    let base_vaddr = manager.next_free_mmio_vaddr;
    let start_page = Page::<Size4KiB>::containing_address(base_vaddr);

    let flags = PageTableFlags::PRESENT
              | PageTableFlags::WRITABLE
              | PageTableFlags::NO_CACHE
              | PageTableFlags::WRITE_THROUGH;

    for i in 0..page_count as u64 {
        let current_page  = start_page + i;
        let current_frame = PhysFrame::containing_address(
            x86_64::PhysAddr::new(phys_aligned + i * 4096)
        );

        let result = unsafe {
            manager.mapper.map_to(current_page, current_frame, flags, &mut manager.dma_allocator)
        };

        match result {
            Ok(flusher) => flusher.flush(),
            Err(e) => panic!("map_mmio: failed to map page {:?}: {:?}", current_page, e),
        }
    }

    // Advance the MMIO bump pointer past the mapped region.
    manager.next_free_mmio_vaddr = base_vaddr + (page_count as u64 * 4096);

    // Return the virtual address that corresponds to the original (possibly
    // non-page-aligned) physical base address.
    base_vaddr + page_offset
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
    region_idx: usize,
    region_offset: u64,
    pub next: usize,
}

impl BootInfoFrameAllocator {
    pub unsafe fn init(memory_map: &'static MemoryRegions) -> Self {
        let mut alloc = BootInfoFrameAllocator {
            memory_map,
            region_idx: 0,
            region_offset: 0,
            next: 0,
        };
        alloc.advance_to_next_usable_region();
        alloc
    }

    fn advance_to_next_usable_region(&mut self) {
        while self.region_idx < self.memory_map.len() {
            if self.memory_map[self.region_idx].kind == MemoryRegionKind::Usable {
                return;
            }
            self.region_idx += 1;
        }
    }

    pub fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> + '_ {
        self.memory_map
            .iter()
            .filter(|r| r.kind == MemoryRegionKind::Usable)
            .flat_map(|r| (r.start..r.end).step_by(4096))
            .map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
    }
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        loop {
            if self.region_idx >= self.memory_map.len() {
                return None;
            }
            let region = &self.memory_map[self.region_idx];
            let region_size = region.end.saturating_sub(region.start);

            if region.kind != MemoryRegionKind::Usable || self.region_offset >= region_size {
                self.region_idx += 1;
                self.region_offset = 0;
                self.advance_to_next_usable_region();
                continue;
            }

            let phys_addr = region.start + self.region_offset;
            self.region_offset += 4096;
            self.next += 1;
            return Some(PhysFrame::containing_address(PhysAddr::new(phys_addr)));
        }
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