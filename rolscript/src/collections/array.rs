use core::fmt::Result as FmtResult;
use core::fmt::Write;
use core::ptr::addr_of_mut;
use core::ptr::NonNull;

use crate::alloc::Allocator;

#[repr(C)]
pub struct FixedArray<T> {
    _len: usize,
    _capacity: usize,
    _data: [T; 1],
}

impl<T> FixedArray<T> {
    fn need_size(count: usize) -> usize {
        use core::mem::size_of;
        size_of::<FixedArray<T>>() + size_of::<T>() * count
    }

    pub unsafe fn init(mut ptr: NonNull<Self>, capacity: usize) {
        let r = ptr.as_mut();
        addr_of_mut!(r._len).write(0);
        addr_of_mut!(r._capacity).write(capacity);
    }

    pub unsafe fn free(allocator: &dyn Allocator, mut ptr: NonNull<Self>) {
        ptr.as_mut().clear();
        let size = Self::need_size(ptr.as_ref()._capacity);
        let align = core::mem::size_of::<usize>();
        allocator.free(ptr.as_ptr() as *mut u8, size, align);
    }

    pub fn new(allocator: &dyn Allocator, capacity: usize) -> Option<NonNull<Self>> {
        unsafe {
            let size = Self::need_size(capacity);
            let align = core::mem::size_of::<usize>();
            let ptr = allocator.alloc(size, align) as *mut Self;
            if ptr.is_null() {
                return None;
            }
            Self::init(NonNull::new_unchecked(ptr), capacity);
            Some(NonNull::new_unchecked(ptr))
        }
    }

    pub fn as_ptr(&self) -> *const T {
        self._data.as_ptr()
    }
    pub fn as_mut_ptr(&mut self) -> *mut T {
        self._data.as_mut_ptr()
    }

    pub fn as_slice(&self) -> &[T] {
        unsafe { core::slice::from_raw_parts(self.as_ptr(), self._len) }
    }
    pub fn as_slice_mut(&mut self) -> &mut [T] {
        unsafe { core::slice::from_raw_parts_mut(self.as_mut_ptr(), self._len) }
    }

    pub fn capacity(&self) -> usize {
        self._capacity
    }
    pub fn len(&self) -> usize {
        self._len
    }

    pub fn push(&mut self, v: T) -> Result<(), T> {
        if self._len == self._capacity {
            Err(v)
        } else {
            unsafe { self.push_unchecked(v) }
            Ok(())
        }
    }

    pub fn pop(&mut self) -> Option<T> {
        if self._len == 0 {
            None
        } else {
            unsafe { Some(self.pop_unchecked()) }
        }
    }

    pub unsafe fn push_unchecked(&mut self, v: T) {
        self.as_mut_ptr().add(self._len).write(v);
        self._len += 1;
    }

    pub unsafe fn pop_unchecked(&mut self) -> T {
        self._len -= 1;
        self.as_mut_ptr().add(self._len).read()
    }

    pub fn insert(&mut self, idx: usize, v: T) -> Result<(), T> {
        if self._len == self._capacity {
            Err(v)
        } else {
            unsafe {
                let len = self._len;
                let ptr = self.as_mut_ptr().add(idx);
                if idx < len {
                    core::ptr::copy(ptr, ptr.add(1), len - idx);
                } else if idx == len {
                } else {
                    return Err(v);
                }
                self._len += 1;
                ptr.write(v);
            }
            Ok(())
        }
    }

    pub fn remove(&mut self, idx: usize) -> Option<T> {
        let len = self._len;

        if idx >= len {
            return None;
        }

        unsafe {
            let ptr = self.as_mut_ptr().add(idx);
            let ret = ptr.read();

            core::ptr::copy(ptr.add(1), ptr, len - idx - 1);
            self._len -= 1;
            Some(ret)
        }
    }

    pub fn get(&self, idx: usize) -> Option<&T> {
        if idx >= self._len {
            None
        } else {
            unsafe {
                let ptr = self.as_ptr().add(idx);
                Some(&*ptr)
            }
        }
    }

    pub fn set(&mut self, idx: usize, v: T) -> Result<T, T> {
        if idx >= self._len {
            Err(v)
        } else {
            unsafe {
                let ptr = self.as_mut_ptr().add(idx);
                let old = ptr.read();
                ptr.write(v);
                Ok(old)
            }
        }
    }

    pub fn clear(&mut self) {
        unsafe {
            let ptr = self.as_mut_ptr();
            for i in 0..self._len {
                ptr.add(i).drop_in_place();
            }
            self._len = 0;
        }
    }

    pub fn append_slice(&mut self, slice: &[T]) -> Result<(), ()>
    where
        T: Copy,
    {
        if self._capacity - self._len >= slice.len() {
            unsafe {
                self.as_mut_ptr()
                    .add(self._len)
                    .copy_from(slice.as_ptr(), slice.len());
                self._len += slice.len();
            }
            Ok(())
        } else {
            Err(())
        }
    }

    pub fn append_with_move(&mut self, other: &mut Self) -> Result<(), ()> {
        if (self._capacity - self._len) < other._capacity {
            return Err(());
        }
        unsafe {
            let self_ptr = self.as_mut_ptr().add(self._len);
            let other_ptr = other.as_mut_ptr();
            for i in 0..other._len {
                let item = other_ptr.add(i).read();
                self_ptr.add(i).write(item);
            }

            self._len += other._len;
            other._len = 0;

            Ok(())
        }
    }
}

impl<T> core::ops::Deref for FixedArray<T> {
    type Target = [T];
    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}
impl<T> core::ops::DerefMut for FixedArray<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_slice_mut()
    }
}

static mut _EMPTY_FIXED_ARRAY: FixedArray<u8> = FixedArray {
    _len: 0,
    _capacity: 0,
    _data: [0],
};

fn empty_fixed_array<'a, T>() -> NonNull<FixedArray<T>> {
    unsafe { NonNull::new_unchecked(addr_of_mut!(_EMPTY_FIXED_ARRAY) as *mut FixedArray<T>) }
}

#[repr(C)]
pub struct Array<T> {
    _allocator: &'static dyn Allocator,
    _inner_array: NonNull<FixedArray<T>>,
}

impl<T> Drop for Array<T> {
    fn drop(&mut self) {
        self._free_fixed(self._inner_array);
    }
}

impl<T> Array<T> {
    pub fn new(allocator: &'static dyn Allocator) -> Self {
        Self {
            _allocator: allocator,
            _inner_array: empty_fixed_array(),
        }
    }

    fn inner(&self) -> &FixedArray<T> {
        unsafe { self._inner_array.as_ref() }
    }

    fn inner_mut(&mut self) -> &mut FixedArray<T> {
        unsafe { self._inner_array.as_mut() }
    }

    fn allocator(&self) -> &dyn Allocator {
        self._allocator
    }

    pub fn capacity(&self) -> usize {
        self.inner().capacity()
    }
    pub fn len(&self) -> usize {
        self.inner().len()
    }

    pub fn as_slice(&self) -> &[T] {
        self.inner().as_slice()
    }
    pub fn as_slice_mut(&mut self) -> &mut [T] {
        self.inner_mut().as_slice_mut()
    }

    fn _free_fixed(&mut self, inner: NonNull<FixedArray<T>>) {
        if inner != empty_fixed_array() {
            unsafe { FixedArray::<T>::free(self.allocator(), inner) }
        }
    }

    pub fn reserve(&mut self, new_capacity: usize) -> Result<(), ()> {
        if new_capacity <= self.capacity() {
            return Ok(());
        }

        let mut new_inner = FixedArray::<T>::new(self.allocator(), new_capacity).ok_or(())?;

        unsafe {
            if let Err(()) = new_inner
                .as_mut()
                .append_with_move(self._inner_array.as_mut())
            {
                FixedArray::<T>::free(self.allocator(), new_inner);
                return Err(());
            }

            let old = core::mem::replace(&mut self._inner_array, new_inner);
            self._free_fixed(old);

            Ok(())
        }
    }

    pub fn resize(&mut self, new_size: usize, default: T) -> Result<(), ()>
    where
        T: Clone,
    {
        let new_cap = if new_size < 8 {
            8
        } else {
            (new_size as f32 * 1.5).ceil() as usize
        };
        self.reserve(new_cap)?;
        if self.len() < new_size {
            #[allow(unused_must_use)]
            for _ in 0..(new_size - self.len()) {
                self.push(default.clone());
            }
            Ok(())
        } else if self.len() > new_size {
            for _ in 0..(self.len() - new_size) {
                self.pop();
            }
            Ok(())
        } else {
            Ok(())
        }
    }

    fn _expansion(&mut self) -> Result<(), ()> {
        let cap = self.capacity();
        let new_cap = if cap < 8 {
            8
        } else {
            (cap as f32 * 1.5).ceil() as usize
        };
        self.reserve(new_cap)
    }

    pub fn push(&mut self, value: T) -> Result<(), T> {
        let cap = self.capacity();

        if self.len() == cap && self._expansion().is_err() {
            return Err(value);
        }

        unsafe {
            self.inner_mut().push_unchecked(value);
            Ok(())
        }
    }

    pub fn pop(&mut self) -> Option<T> {
        self.inner_mut().pop()
    }

    pub fn insert(&mut self, idx: usize, value: T) -> Result<(), T> {
        let cap = self.capacity();

        if self.len() == cap && self._expansion().is_err() {
            return Err(value);
        }

        self.inner_mut().insert(idx, value)
    }

    pub fn remove(&mut self, idx: usize) -> Option<T> {
        self.inner_mut().remove(idx)
    }

    pub fn get(&self, idx: usize) -> Option<&T> {
        self.inner().get(idx)
    }

    pub fn set(&mut self, idx: usize, value: T) -> Result<T, T> {
        self.inner_mut().set(idx, value)
    }

    pub fn append_slice(&mut self, slice: &[T]) -> Result<(), ()>
    where
        T: Copy,
    {
        if self.capacity() - self.len() < slice.len() {
            self.reserve(self.capacity() + slice.len())?;
        }

        self.inner_mut().append_slice(slice)
    }

    pub fn iter<'a>(&'a self) -> core::slice::Iter<'a, T> {
        self.as_slice().iter()
    }
}

impl Array<u8> {
    pub unsafe fn as_str_unchecked(&self) -> &str {
        core::str::from_utf8_unchecked(self.as_slice())
    }

    pub fn append_str(&mut self, s: &str) -> Result<(), ()> {
        self.append_slice(s.as_bytes())
    }
}

impl Write for Array<u8> {
    fn write_str(&mut self, s: &str) -> FmtResult {
        self.append_slice(s.as_bytes())
            .map_err(|_| Default::default())
    }
}

#[cfg(test)]
mod test {

    use super::super::super::alloc;
    use super::*;

    fn allocator() -> &'static dyn Allocator {
        alloc::default_allocator()
    }

    #[test]
    fn test_fixed_array() {
        let array_ptr = FixedArray::<i32>::new(allocator(), 10);
        assert!(array_ptr.is_some());
        let mut array_ptr = array_ptr.unwrap();

        let array = unsafe { array_ptr.as_mut() };
        for i in 0..10 {
            assert!(array.push(i).is_ok());
            assert_eq!(array.len(), i as usize + 1);
        }

        assert_eq!(array.as_slice(), &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);

        assert_eq!(array.remove(0), Some(0));
        assert_eq!(array.len(), 9);
        assert_eq!(array.as_slice(), &[1, 2, 3, 4, 5, 6, 7, 8, 9]);

        assert_eq!(array.remove(2), Some(3));
        assert_eq!(array.len(), 8);
        assert_eq!(array.as_slice(), &[1, 2, 4, 5, 6, 7, 8, 9]);

        assert_eq!(array.remove(array.len() - 1), Some(9));
        assert_eq!(array.len(), 7);
        assert_eq!(array.as_slice(), &[1, 2, 4, 5, 6, 7, 8]);

        assert_eq!(array.insert(2, 3), Ok(()));
        assert_eq!(array.len(), 8);
        assert_eq!(array.as_slice(), &[1, 2, 3, 4, 5, 6, 7, 8]);

        for i in (1..9).rev() {
            assert_eq!(array.pop(), Some(i));
            assert_eq!(array.len(), i as usize - 1);
        }

        unsafe { FixedArray::free(allocator(), array_ptr) };
    }

    #[test]
    fn test_array() {
        let mut array = Array::<i32>::new(allocator());

        for i in 0..100 {
            assert!(array.push(i).is_ok());
            assert_eq!(array.len(), i as usize + 1);
        }

        assert_eq!(array.remove(0), Some(0));
        assert_eq!(array.len(), 99);

        assert_eq!(array.remove(2), Some(3));
        assert_eq!(array.len(), 98);

        assert_eq!(array.remove(array.len() - 1), Some(99));
        assert_eq!(array.len(), 97);

        assert_eq!(array.insert(2, 3), Ok(()));
        assert_eq!(array.len(), 98);

        for i in (1..99).rev() {
            assert_eq!(array.pop(), Some(i));
            assert_eq!(array.len(), i as usize - 1);
        }
    }
}
