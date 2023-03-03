#![allow(non_snake_case)]

use core::mem::size_of;
use core::ptr::addr_of_mut;
use core::ptr::NonNull;

use crate::alloc::Allocator;
use crate::collections::Array;

use crate::runtime::*;

use crate::error::*;
use crate::runtime_error_fmt;

use crate::number::*;
use crate::option::ROption;
use crate::string::RString;
use crate::type_::*;
use crate::value::*;

use crate::builtin::*;

#[repr(C)]
pub struct RArray {
    _header: GcHeader,
    _array: Array<RValue>,
}

impl RArray {
    unsafe fn init(mut ptr: NonNull<Self>, allocator: &'static dyn Allocator) {
        let arr = Array::<RValue>::new(allocator);
        addr_of_mut!(ptr.as_mut()._array).write(arr);
    }

    unsafe fn _drop(&mut self) {
        addr_of_mut!(self._array).drop_in_place();
    }

    pub fn new() -> Result<Ref<Self>, Error> {
        let tp = array_type().clone();
        unsafe {
            let v = new_gc_obj(size_of::<Self>(), tp)?.cast::<Self>();
            Self::init(v.as_nonnull_ptr(), allocator());
            Ok(v)
        }
    }

    pub fn as_slice(&self) -> &[RValue] {
        self._array.as_slice()
    }

    pub fn as_slice_mut(&mut self) -> &mut [RValue] {
        self._array.as_slice_mut()
    }

    pub fn len(&self) -> usize {
        self._array.len()
    }

    pub fn capacity(&self) -> usize {
        self._array.capacity()
    }

    pub fn get(&self, index: Int) -> Option<&RValue> {
        unsafe {
            let len = self._array.len() as Int;
            if index >= -len && index < 0 {
                let index = (len + index) as usize;
                Some(self.as_slice().get_unchecked(index))
            } else if index >= 0 && index < len {
                let index = index as usize;
                Some(self.as_slice().get_unchecked(index))
            } else {
                None
            }
        }
    }

    pub fn set(&mut self, index: Int, value: RValue) -> Result<RValue, Error> {
        use core::mem::replace;
        unsafe {
            let len = self._array.len() as Int;
            if index >= -len && index < 0 {
                let index = (len + index) as usize;
                let old = replace(self.as_slice_mut().get_unchecked_mut(index), value);
                Ok(old)
            } else if index >= 0 && index < len {
                let index = index as usize;
                let old = replace(self.as_slice_mut().get_unchecked_mut(index), value);
                Ok(old)
            } else {
                Err(Error::new_outofrange())
            }
        }
    }

    pub fn push(&mut self, value: RValue) -> Result<(), Error> {
        self._array.push(value).map_err(|_| Error::OutOfMemory)
    }

    pub fn pop(&mut self) -> Option<RValue> {
        self._array.pop()
    }
}

pub(crate) fn _init_type_array(mut tp: Ref<RType>) -> Result<(), Error> {
    tp.with_visit(array__visit);

    tp.with_destory(array__destory);

    tp.with_eq(default_value_eq);
    tp.with_hash(default_value_hash);
    tp.with_str(array__to_string);

    tp.with_get_item(array__get_item);
    tp.with_set_item(array__set_item);

    tp.with_iter(array__iter);

    tp.add_method_str_light("len", array__len)?;

    Ok(())
}

fn array__visit(visitor: &mut dyn Visitor, value_ptr: NonNull<GcHeader>) {
    unsafe {
        let arr = value_ptr.cast::<RArray>();
        for v in arr.as_ref().as_slice() {
            visitor.visit_value(v);
        }
    }
}

fn array__destory(value: &RValue) -> Result<(), Error> {
    unsafe {
        let mut arr = value.expect_cast::<RArray>(array_type())?;
        arr._drop();
        Ok(())
    }
}

fn array__to_string(value: &RValue) -> Result<Ref<RString>, Error> {
    use core::fmt::Write;

    fn _e(value: &RValue) -> Error {
        match default_value_str(value) {
            Ok(s) => runtime_error_fmt!("{} connot to string", s.as_str()),
            Err(e) => e,
        }
    }

    let array = unsafe { value.expect_cast::<RArray>(array_type())? };
    let mut buf = Array::new(allocator());
    if array.len() == 0 {
        RString::new("[]")
    } else {
        let vs = value_repr(&array.as_slice()[0])?;
        write!(&mut buf, "[{}", vs.as_str()).map_err(|_| _e(value))?;

        for v in &array.as_slice()[1..] {
            let vs = value_repr(v)?;
            write!(&mut buf, ", {}", vs.as_str()).map_err(|_| _e(value))?;
        }

        write!(&mut buf, "]").map_err(|_| _e(value))?;

        unsafe { RString::new(buf.as_str_unchecked()) }
    }
}

fn array__get_item(value: &RValue, index: &RValue) -> Result<RValue, Error> {
    let array = unsafe { value.expect_cast::<RArray>(array_type())? };
    let index = unsafe { index.expect_cast::<RInt>(int_type())? };
    if let Some(v) = array.get(index.as_number()) {
        Ok(v.clone())
    } else {
        Err(Error::new_outofrange())
    }
}

fn array__set_item(value: &RValue, index: &RValue, item_value: &RValue) -> Result<(), Error> {
    let mut array = unsafe { value.expect_cast::<RArray>(array_type())? };
    let index = unsafe { index.expect_cast::<RInt>(int_type())? };
    array.set(index.as_number(), item_value.clone()).map(|_| ())
}

fn array__len(this: &RValue, _args: &[RValue]) -> Result<RValue, Error> {
    let array = unsafe { this.expect_cast::<RArray>(array_type())? };
    Ok(RInt::new(array.len() as isize)?.cast_value())
}

fn array__iter(value: &RValue) -> Result<RValue, Error> {
    let arr = unsafe { value.expect_cast::<RArray>(array_type())? };
    let it = RArrayIter::new(&arr)?;
    Ok(it.cast_value())
}

pub struct RArrayIter {
    _header: GcHeader,
    _array: Ref<RArray>,
    _current: usize,
}

impl RArrayIter {
    pub fn new(array: &Ref<RArray>) -> Result<Ref<Self>, Error> {
        unsafe {
            let mut v =
                new_gc_obj(size_of::<RArrayIter>(), array_iter_type().clone())?.cast::<Self>();
            addr_of_mut!(v._array).write(array.clone());
            addr_of_mut!(v._current).write(0);
            Ok(v)
        }
    }

    pub fn next(&mut self) -> Result<Option<RValue>, Error> {
        let i = self._current;
        self._current += 1;
        Ok(self._array.as_slice().get(i).cloned())
    }
}

pub(crate) fn _init_type_arrayiter(mut tp: Ref<RType>) -> Result<(), Error> {
    tp.with_str(default_value_str);
    tp.with_hash(default_value_hash);
    tp.with_eq(default_value_eq);

    tp.with_next(array_iter__next);

    Ok(())
}

fn array_iter__next(value: &RValue) -> Result<Ref<ROption>, Error> {
    let mut it = unsafe { value.expect_cast::<RArrayIter>(array_iter_type())? };
    let nv = it.next()?;
    ROption::new(nv)
}
