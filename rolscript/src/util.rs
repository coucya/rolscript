#![allow(dead_code)]

use crate::alloc::Allocator;
use crate::collections::HashMap;

use crate::error::*;
use crate::runtime_error_fmt;

use crate::string::RString;
use crate::value::*;

pub(crate) unsafe fn clone_mut_ref<T: ?Sized>(ref_: &mut T) -> &'static mut T {
    unsafe {
        let ptr = ref_ as *mut T;
        &mut *ptr
    }
}

pub(crate) struct StringMap<V>(HashMap<Ref<RString>, V>);

#[allow(dead_code)]
impl<V> StringMap<V> {
    pub(crate) fn new(allocator: &'static dyn Allocator) -> Self {
        StringMap(HashMap::new(allocator))
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.0.capacity()
    }

    pub(crate) fn insert(&mut self, key: Ref<RString>, value: V) -> Result<Option<V>, Error> {
        let mut hf = |s: &Ref<RString>| Ok(s.as_ptr() as usize);
        let mut ef = |a: &Ref<RString>, b: &Ref<RString>| Ok(a.as_ptr() == b.as_ptr());
        self.0
            .insert_when(key, value, &mut hf, &mut ef)
            .map_err(|_| Error::new_outofmemory())
    }

    pub(crate) fn remove(&mut self, key: &Ref<RString>) -> Option<V> {
        let mut hf = |s: &Ref<RString>| Ok(s.as_ptr() as usize);
        let mut ef = |a: &Ref<RString>, b: &Ref<RString>| Ok(a.as_ptr() == b.as_ptr());
        self.0.remove_when(key, &mut hf, &mut ef)
    }

    pub(crate) fn get(&self, key: &Ref<RString>) -> Option<&V> {
        let mut hf = |s: &Ref<RString>| Ok(s.as_ptr() as usize);
        let mut ef = |a: &Ref<RString>, b: &Ref<RString>| Ok(a.as_ptr() == b.as_ptr());
        self.0.get_when(key, &mut hf, &mut ef)
    }

    pub(crate) fn get_key_value(&self, key: &Ref<RString>) -> Option<(&Ref<RString>, &V)> {
        let mut hf = |s: &Ref<RString>| Ok(s.as_ptr() as usize);
        let mut ef = |a: &Ref<RString>, b: &Ref<RString>| Ok(a.as_ptr() == b.as_ptr());
        self.0.get_key_value_when(key, &mut hf, &mut ef)
    }

    pub fn contains_key(&self, key: &Ref<RString>) -> bool {
        let mut hf = |s: &Ref<RString>| Ok(s.as_ptr() as usize);
        let mut ef = |a: &Ref<RString>, b: &Ref<RString>| Ok(a.as_ptr() == b.as_ptr());
        self.0.contains_key_when(key, &mut hf, &mut ef)
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (&Ref<RString>, &V)> {
        self.0.iter()
    }
}

pub(crate) struct ValueMap<V>(HashMap<RValue, V>);

#[allow(dead_code)]
impl<V> ValueMap<V> {
    pub(crate) fn new(allocator: &'static dyn Allocator) -> Self {
        ValueMap(HashMap::new(allocator))
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.0.capacity()
    }

    pub(crate) fn insert(&mut self, key: RValue, value: V) -> Result<Option<V>, Error> {
        let mut hf = |s: &RValue| value_hash(s).map(|v| v as usize).map_err(|_| ());
        let mut ef = |a: &RValue, b: &RValue| value_eq(a, b).map_err(|_| ());
        self.0
            .insert_when(key, value, &mut hf, &mut ef)
            .map_err(|_| Error::new_outofmemory())
    }

    pub(crate) fn remove(&mut self, key: &RValue) -> Option<V> {
        let mut hf = |s: &RValue| value_hash(s).map(|v| v as usize).map_err(|_| ());
        let mut ef = |a: &RValue, b: &RValue| value_eq(a, b).map_err(|_| ());
        self.0.remove_when(key, &mut hf, &mut ef)
    }

    pub(crate) fn get(&self, key: &RValue) -> Option<&V> {
        let mut hf = |s: &RValue| value_hash(s).map(|v| v as usize).map_err(|_| ());
        let mut ef = |a: &RValue, b: &RValue| value_eq(a, b).map_err(|_| ());
        self.0.get_when(key, &mut hf, &mut ef)
    }

    pub(crate) fn get_key_value(&self, key: &RValue) -> Option<(&RValue, &V)> {
        let mut hf = |s: &RValue| value_hash(s).map(|v| v as usize).map_err(|_| ());
        let mut ef = |a: &RValue, b: &RValue| value_eq(a, b).map_err(|_| ());
        self.0.get_key_value_when(key, &mut hf, &mut ef)
    }

    #[inline]
    pub fn contains_key(&self, key: &RValue) -> bool {
        let mut hf = |s: &RValue| value_hash(s).map(|v| v as usize).map_err(|_| ());
        let mut ef = |a: &RValue, b: &RValue| value_eq(a, b).map_err(|_| ());
        self.0.contains_key_when(key, &mut hf, &mut ef)
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (&RValue, &V)> {
        self.0.iter()
    }
}

pub fn expect_arg1(args: &[RValue]) -> Result<RValue, Error> {
    if args.len() >= 1 {
        unsafe { Ok(args.get_unchecked(0).clone()) }
    } else {
        Err(runtime_error_fmt!(
            "expected 1 parameter, but gave {}",
            args.len()
        ))
    }
}

pub fn expect_arg2(args: &[RValue]) -> Result<(RValue, RValue), Error> {
    if args.len() >= 2 {
        unsafe { Ok((args.get_unchecked(0).clone(), args.get_unchecked(2).clone())) }
    } else {
        Err(runtime_error_fmt!(
            "expected 2 parameter, but gave {}",
            args.len()
        ))
    }
}

#[macro_export]
macro_rules! nonnull_of {
    ($expr: expr) => {{
        use core::ptr::addr_of;
        use core::ptr::NonNull;

        #[allow(unused_unsafe)]
        unsafe {
            NonNull::new_unchecked(addr_of!($expr) as _)
        }
    }};
}
