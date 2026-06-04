use fixed_size_blocks::FixedSizeBlockAllocator;
use bootloader_api::BootInfo;
use crate::memory::{MEMORY_MANAGER, MemoryKernelManager};

#[global_allocator]
static ALLOCATOR: Locked<FixedSizeBlockAllocator> = Locked::new(FixedSizeBlockAllocator::new());

use alloc::alloc::{GlobalAlloc, Layout};
use core::ptr::null_mut;

pub mod bump;
pub mod linked_list;
pub mod fixed_size_blocks;

pub struct Dummy;

unsafe impl GlobalAlloc for Dummy {
    unsafe fn alloc(&self, _layout: Layout) -> *mut u8 {
        null_mut()
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        panic!("dealloc should be never called")
    }
}

pub const HEAP_START: usize = 0x_4444_4444_0000;
pub const HEAP_SIZE: usize = 100 * 1024; 

use x86_64::{
    structures::paging::{
        mapper::MapToError, FrameAllocator, Mapper, Page, PageTableFlags, Size4KiB,
    },
    VirtAddr,
};

pub fn init_heap(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> Result<(), MapToError<Size4KiB>> {
    let page_range = {
        let heap_start = VirtAddr::new(HEAP_START as u64);
        let heap_end = heap_start + HEAP_SIZE - 1u64;
        let heap_start_page = Page::containing_address(heap_start);
        let heap_end_page = Page::containing_address(heap_end);
        Page::range_inclusive(heap_start_page, heap_end_page)
    };

    for page in page_range {
        let frame = frame_allocator
            .allocate_frame()
            .ok_or(MapToError::FrameAllocationFailed)?;
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        unsafe {
            mapper.map_to(page, frame, flags, frame_allocator)?.flush()
        };
    }

    unsafe {
        ALLOCATOR.lock().init(HEAP_START, HEAP_SIZE);
    }

    Ok(())
}

pub fn alloc_init(boot_info: &'static BootInfo){
    use crate::memory::{self, BootInfoFrameAllocator, BitmapFrameAllocator};

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset.into_option()
        .expect("Physical memory mapping was not enabled in BOOTLOADER_CONFIG"));
    let mut mapper = memory::init(phys_mem_offset);
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_regions) };

    init_heap(&mut mapper, &mut frame_allocator).expect("heap initialization failed");

    let total_frames = 262144; 
    let bitmap_size_bytes = total_frames / 8; // 32 KiB

    let bitmap_frame = frame_allocator.allocate_frame().expect("Failed to allocate frame for bitmap tracking");
    
    let bitmap_vaddr = phys_mem_offset + bitmap_frame.start_address().as_u64();
    let bitmap_slice = unsafe {
        core::slice::from_raw_parts_mut(bitmap_vaddr.as_mut_ptr::<u8>(), bitmap_size_bytes)
    };

    let mut dma_allocator = unsafe { 
        BitmapFrameAllocator::new(bitmap_slice, total_frames) 
    };
    
    dma_allocator.fill_from_boot_allocator(&frame_allocator);

    unsafe {
        MEMORY_MANAGER = Some(MemoryKernelManager {
            mapper,
            dma_allocator,
            next_free_dma_vaddr: VirtAddr::new(0xFFFF_A000_0000_0000),
        });
    }
}

pub struct Locked<A> {
    inner: spin::Mutex<A>,
}

impl<A> Locked<A> {
    pub const fn new(inner: A) -> Self {
        Locked {
            inner: spin::Mutex::new(inner),
        }
    }

    pub fn lock(&self) -> spin::MutexGuard<A> {
        self.inner.lock()
    }
}

fn align_up(addr: usize, align: usize) -> usize {
    let remainder = addr % align;
    if remainder == 0 {
        addr
    } else {
        addr - remainder + align
    }
}

/*
fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}
*/