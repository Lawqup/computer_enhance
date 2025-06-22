use std::{alloc::GlobalAlloc, ffi::c_void, ptr::null_mut};

pub struct MmapAllocator;

#[global_allocator]
pub static ALLOCATOR: MmapAllocator = MmapAllocator;

unsafe impl GlobalAlloc for MmapAllocator {
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        let ptr =
            match libc::mmap(
                null_mut(),
                layout.size(),
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED | libc::MAP_ANONYMOUS,
                -1,
                0,
            ) {
                libc::MAP_FAILED => panic!("Failed to map memory"),
                ptr => ptr as *mut u8,
            };

        ptr
    }
    
    unsafe fn alloc_zeroed(&self, layout: std::alloc::Layout) -> *mut u8 {
        // The flags passed into mmap in alloc cause this to be zeroed
        // The default zeroed implementation will differ as it will try and write 0s, thus
        // effectively prefetching uninintentionally
        self.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: std::alloc::Layout) {
        libc::munmap(ptr as *mut c_void, layout.size());
    }
}
