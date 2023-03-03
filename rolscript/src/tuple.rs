#![allow(non_snake_case)]

use core::mem::size_of;
use core::ptr::addr_of_mut;
use core::ptr::NonNull;

use crate::runtime::*;

use crate::error::*;
use crate::runtime_error_fmt;

use crate::number::*;
use crate::option::*;
use crate::string::RString;
use crate::type_::*;
use crate::value::*;

use crate::builtin::*;

#[repr(C)]
pub struct RTuple {
    _header: GcHeader,
    _len: usize,
    _items: [RValue; 1],
}

impl RTuple {
    fn need_size(len: usize) -> usize {
        size_of::<Self>() + size_of::<RValue>() * len
    }

    unsafe fn init(mut ptr: NonNull<Self>, len: usize) {
        let r = ptr.as_mut();
        addr_of_mut!(r._len).write(len);

        let null_value = null().cast_value();
        let item_ptr = r._items.as_mut_ptr();
        for i in 0..len {
            item_ptr.add(i).write(null_value.clone());
        }
    }

    unsafe fn init_from_slice(mut ptr: NonNull<Self>, slice: &[RValue]) {
        let r = ptr.as_mut();
        addr_of_mut!(r._len).write(slice.len());

        let item_ptr = r._items.as_mut_ptr();
        for (i, v) in slice.iter().enumerate() {
            item_ptr.add(i).write(v.clone());
        }
    }

    pub fn new(len: usize) -> Result<Ref<Self>, Error> {
        let tp = tuple_type().clone();
        unsafe {
            let v = new_gc_obj(Self::need_size(len), tp)?.cast::<Self>();
            Self::init(v.as_nonnull_ptr(), len);
            Ok(v)
        }
    }

    pub fn from_slice(slice: &[RValue]) -> Result<Ref<Self>, Error> {
        let tp = tuple_type().clone();
        unsafe {
            let v = new_gc_obj(Self::need_size(slice.len()), tp)?.cast::<Self>();
            Self::init_from_slice(v.as_nonnull_ptr(), slice);
            Ok(v)
        }
    }

    pub fn len(&self) -> usize {
        self._len
    }

    pub fn as_slice(&self) -> &[RValue] {
        unsafe { core::slice::from_raw_parts(self._items.as_ptr(), self._len) }
    }

    pub fn as_slice_mut(&mut self) -> &mut [RValue] {
        unsafe { core::slice::from_raw_parts_mut(self._items.as_mut_ptr(), self._len) }
    }

    pub fn get(&self, index: Int) -> Option<&RValue> {
        unsafe {
            if index >= -(self._len as Int) && index < 0 {
                let index = (self._len as Int + index) as usize;
                Some(self.as_slice().get_unchecked(index))
            } else if index >= 0 && index < (self._len as Int) {
                let index = index as usize;
                Some(self.as_slice().get_unchecked(index))
            } else {
                None
            }
        }
    }

    pub fn set(&mut self, index: isize, value: RValue) -> Result<RValue, Error> {
        use core::mem::replace;
        unsafe {
            if index >= -(self._len as Int) && index < 0 {
                let index = (self._len as Int + index) as usize;
                let old = replace(self.as_slice_mut().get_unchecked_mut(index), value);
                Ok(old)
            } else if index >= 0 && index < (self._len as Int) {
                let index = index as usize;
                let old = replace(self.as_slice_mut().get_unchecked_mut(index), value);
                Ok(old)
            } else {
                Err(Error::new_outofmemory())
            }
        }
    }
}

pub(crate) fn _init_type_tuple(mut tp: Ref<RType>) -> Result<(), Error> {
    tp.with_visit(tuple__visit);

    tp.with_destory(tuple__destory);

    tp.with_eq(tuple__eq);
    tp.with_hash(default_value_hash);
    tp.with_str(tuple__to_string);

    tp.with_get_item(tuple__get_item);
    tp.with_set_item(tuple__set_item);

    tp.with_iter(tuple__iter);

    tp.add_method_str_light("len", |instance, _args| {
        let tuple = unsafe { instance.expect_cast::<RTuple>(tuple_type())? };
        Ok(RInt::new(tuple.len() as Int)?.cast_value())
    })?;

    Ok(())
}

fn tuple__visit(visitor: &mut dyn Visitor, value_ptr: NonNull<GcHeader>) {
    unsafe {
        let arr = value_ptr.cast::<RTuple>();
        for v in arr.as_ref().as_slice() {
            visitor.visit_value(v);
        }
    }
}

fn tuple__destory(value: &RValue) -> Result<(), Error> {
    let mut tuple = unsafe { value.expect_cast::<RTuple>(tuple_type())? };
    for i in 0..tuple._len {
        unsafe { tuple.as_mut_ptr().add(i).drop_in_place() }
    }
    Ok(())
}

fn tuple__to_string(value: &RValue) -> Result<Ref<RString>, Error> {
    use crate::collections::Array;
    use core::fmt::Write;

    fn _e(value: &RValue) -> Error {
        match default_value_str(value) {
            Ok(s) => runtime_error_fmt!("{} connot to string", s.as_str()),
            Err(e) => e,
        }
    }

    let tuple = unsafe { value.expect_cast::<RTuple>(tuple_type())? };
    let mut buf = Array::new(allocator());
    if tuple.len() == 0 {
        RString::new("()")
    } else {
        let vs = value_repr(&tuple.as_slice()[0])?;
        write!(&mut buf, "({}", vs.as_str()).map_err(|_| _e(value))?;

        for v in &tuple.as_slice()[1..] {
            let vs = value_repr(v)?;
            write!(&mut buf, ", {}", vs.as_str()).map_err(|_| _e(value))?;
        }

        write!(&mut buf, ")").map_err(|_| _e(value))?;

        unsafe { RString::new(buf.as_str_unchecked()) }
    }
}

fn tuple__eq(a: &RValue, b: &RValue) -> Result<bool, Error> {
    unsafe {
        if b.is_type(a.get_type()) {
            let at = a.expect_cast::<RTuple>(tuple_type())?;
            let bt = b.expect_cast::<RTuple>(tuple_type())?;
            if at.len() == bt.len() {
                let iter = at.as_slice().iter().zip(bt.as_slice().iter());
                for (a, b) in iter {
                    if !value_eq(a, b)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            } else {
                Ok(false)
            }
        } else {
            Ok(false)
        }
    }
}

fn tuple__get_item(value: &RValue, index: &RValue) -> Result<RValue, Error> {
    let tuple = unsafe { value.expect_cast::<RTuple>(tuple_type())? };
    let index = unsafe { index.expect_cast::<RInt>(int_type())? };
    if let Some(v) = tuple.get(index.as_number()) {
        Ok(v.clone())
    } else {
        Err(Error::new_outofrange())
    }
}

fn tuple__set_item(value: &RValue, index: &RValue, item_value: &RValue) -> Result<(), Error> {
    let mut tuple = unsafe { value.expect_cast::<RTuple>(tuple_type())? };
    let index = unsafe { index.expect_cast::<RInt>(int_type())? };
    tuple.set(index.as_number(), item_value.clone()).map(|_| ())
}

fn tuple__iter(value: &RValue) -> Result<RValue, Error> {
    let arr = unsafe { value.expect_cast::<RTuple>(tuple_type())? };
    let it = RTupleIter::new(&arr)?;
    Ok(it.cast_value())
}

pub struct RTupleIter {
    _header: GcHeader,
    _tuple: Ref<RTuple>,
    _current: usize,
}

impl RTupleIter {
    pub fn new(tuple: &Ref<RTuple>) -> Result<Ref<Self>, Error> {
        unsafe {
            let mut v =
                new_gc_obj(size_of::<RTupleIter>(), tuple_iter_type().clone())?.cast::<Self>();
            addr_of_mut!(v._tuple).write(tuple.clone());
            addr_of_mut!(v._current).write(0);
            Ok(v)
        }
    }

    pub fn next(&mut self) -> Result<Option<RValue>, Error> {
        let i = self._current;
        self._current += 1;
        Ok(self._tuple.as_slice().get(i).cloned())
    }
}

pub(crate) fn _init_type_tupleiter(mut tp: Ref<RType>) -> Result<(), Error> {
    tp.with_str(default_value_str);
    tp.with_hash(default_value_hash);
    tp.with_eq(default_value_eq);

    tp.with_next(tuple_iter__next);

    Ok(())
}

fn tuple_iter__next(value: &RValue) -> Result<Ref<ROption>, Error> {
    let mut it = unsafe { value.expect_cast::<RTupleIter>(tuple_iter_type())? };
    let nv = it.next()?;
    ROption::new(nv)
}
