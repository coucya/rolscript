pub trait Allocator {
    unsafe fn alloc(&self, size: usize, align: usize) -> *mut u8;
    unsafe fn free(&self, ptr: *mut u8, size: usize, align: usize);

    unsafe fn alloc_zeroed(&self, size: usize, align: usize) -> *mut u8 {
        unsafe {
            let ptr = self.alloc(size, align);
            if !ptr.is_null() {
                ptr.write_bytes(0, size);
            }
            ptr
        }
    }

    unsafe fn alloc_block(&self, size: usize) -> *mut u8 {
        let align = core::mem::size_of::<usize>();
        let ptr = self.alloc(size, align);
        ptr
    }

    unsafe fn alloc_block_zeroed(&self, size: usize) -> *mut u8 {
        let align = core::mem::size_of::<usize>();
        let ptr = self.alloc(size, align);
        if !ptr.is_null() {
            ptr.write_bytes(0, size);
        }
        ptr
    }

    unsafe fn free_block(&self, ptr: *mut u8, size: usize) {
        let align = core::mem::size_of::<usize>();
        self.free(ptr, size, align);
    }
}

impl<T: Allocator> Allocator for &T {
    unsafe fn alloc(&self, size: usize, align: usize) -> *mut u8 {
        (**self).alloc(size, align)
    }
    unsafe fn free(&self, ptr: *mut u8, size: usize, align: usize) {
        (**self).free(ptr, size, align)
    }
}

struct Global;
impl Allocator for Global {
    unsafe fn alloc(&self, size: usize, align: usize) -> *mut u8 {
        use std::alloc::*;
        let layout = Layout::from_size_align_unchecked(size, align);
        alloc(layout)
    }
    unsafe fn free(&self, ptr: *mut u8, size: usize, align: usize) {
        use std::alloc::*;
        let layout = Layout::from_size_align_unchecked(size, align);
        dealloc(ptr, layout);
    }
}

static mut GLOBAL: Global = Global {};

pub fn default_allocator() -> &'static mut dyn Allocator {
    unsafe { &mut GLOBAL }
}
