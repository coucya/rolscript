#![allow(non_snake_case)]

use core::fmt::Arguments as FmtArguments;
use core::fmt::Result as FmtResult;
use core::fmt::Write;
use core::fmt::{Debug, Formatter};
use core::hash::{Hash, Hasher};
use core::mem::size_of;
use core::ptr::addr_of_mut;
use core::ptr::NonNull;

use crate::alloc::Allocator;
use crate::collections::Array;
use crate::collections::HashMap;

use crate::runtime::*;

use crate::op::*;

use crate::error::*;
use crate::runtime_error_fmt;

use crate::number::*;
use crate::type_::*;
use crate::value::*;

use crate::builtin::*;

use crate::util::expect_arg1;

#[repr(C)]
pub struct RString {
    _haeder: GcHeader,
    _len: usize,
    _str: [u8; 1],
}

impl RString {
    pub(crate) fn need_size(s: &str) -> usize {
        size_of::<Self>() + s.len()
    }

    pub(crate) unsafe fn init(mut ptr: NonNull<Self>, s: &str) {
        addr_of_mut!(ptr.as_mut()._len).write(s.len());

        let str_ptr = ptr.as_mut()._str.as_mut_ptr();
        str_ptr.copy_from(s.as_ptr(), s.len());

        // C样式字符串的尾随0。
        str_ptr.add(s.len()).write(0);
    }

    pub fn new(s: &str) -> Result<Ref<Self>, Error> {
        runtime().string_pool_get(s)
    }

    pub fn format(args: FmtArguments) -> Result<Ref<Self>, Error> {
        let mut tmp = Array::<u8>::new(allocator());
        tmp.write_fmt(args).map_err(|_| Error::new_outofmemory())?;
        let tmp_str = unsafe { tmp.as_str_unchecked() };
        runtime().string_pool_get(tmp_str)
    }

    pub fn as_str(&self) -> &str {
        unsafe {
            let ptr = self._str.as_ptr();
            let slice = core::slice::from_raw_parts(ptr, self._len);
            core::str::from_utf8_unchecked(slice)
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        unsafe {
            let ptr = self._str.as_ptr();
            core::slice::from_raw_parts(ptr, self._len)
        }
    }

    pub fn len(&self) -> usize {
        self._len
    }
}

impl core::convert::AsRef<str> for Ref<RString> {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl core::convert::AsRef<[u8]> for Ref<RString> {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl Debug for Ref<RString> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_str(self.as_str())
    }
}

struct StringPoolItem(Ref<RString>);

impl Hash for StringPoolItem {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.as_str().hash(state)
    }
}
impl Eq for StringPoolItem {}
impl PartialEq for StringPoolItem {
    fn eq(&self, other: &Self) -> bool {
        self.0.as_str() == other.0.as_str()
    }
    fn ne(&self, other: &Self) -> bool {
        self.0.as_str() != other.0.as_str()
    }
}
impl core::borrow::Borrow<str> for StringPoolItem {
    fn borrow(&self) -> &str {
        self.0.as_str()
    }
}

pub(crate) struct StringPool {
    _pool: HashMap<StringPoolItem, ()>,
}

#[allow(dead_code)]
impl StringPool {
    pub(crate) fn new(allocator: &'static dyn Allocator) -> Self {
        Self {
            _pool: HashMap::new(allocator),
        }
    }

    pub(crate) fn has(&self, string: &str) -> bool {
        self._pool.get_key_value(string).is_some()
    }

    pub(crate) fn get(&self, string: &str) -> Option<Ref<RString>> {
        self._pool.get_key_value(string).map(|(k, _)| k.0.clone())
    }

    pub(crate) fn add(&mut self, string: Ref<RString>) -> Result<(), Error> {
        self._pool
            .insert(StringPoolItem(string), ())
            .map_err(|_| Error::new_outofmemory())?;
        Ok(())
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &Ref<RString>> {
        self._pool.iter().map(|(k, _)| &k.0)
    }
}

pub(crate) fn _init_type_string(mut tp: Ref<RType>) -> Result<(), Error> {
    tp.with_new(string__new);

    tp.with_eq(string__eq);
    tp.with_cmp(string__cmp);
    tp.with_hash(string__hash);
    tp.with_str(string__to_string);

    tp.with_arith(ArithOp::Add, stirng__add);

    tp.add_method_str_light("len", stirng__len)?;

    Ok(())
}

fn string__new(_tp: &Ref<RType>, args: &[RValue]) -> Result<RValue, Error> {
    let arg = expect_arg1(args)?;
    if arg.is_type(string_type()) {
        Ok(arg)
    } else {
        value_str(&arg).map(|v| v.cast_value())
    }
}

fn string__hash(v: &RValue) -> Result<Int, Error> {
    Ok(v.as_ptr() as usize as isize)
}

fn string__eq(a: &RValue, b: &RValue) -> Result<bool, Error> {
    let atp = a.get_type();
    let btp = b.get_type();
    Ok(Ref::ptr_eq(atp, btp) && Ref::ptr_eq(a, b))
}

fn string__cmp(a: &RValue, b: &RValue) -> Result<Int, Error> {
    let tp = string_type();
    unsafe {
        let aa = a.expect_cast::<RString>(tp)?;
        let bb = b.expect_cast::<RString>(tp)?;
        let n = aa.as_str().cmp(bb.as_str()) as Int;
        Ok(n)
    }
}

fn string__to_string(v: &RValue) -> Result<Ref<RString>, Error> {
    if v.is_type(&string_type()) {
        unsafe { Ok(v.clone().cast()) }
    } else {
        Err(runtime_error_fmt!("{:?} is not a string", v))
    }
}

fn stirng__add(a: &RValue, b: &RValue) -> Result<RValue, Error> {
    let as_ = unsafe { a.expect_cast::<RString>(string_type())? };
    let bs = unsafe { b.expect_cast::<RString>(string_type())? };

    let mut buf = Array::new(allocator());
    buf.reserve(as_.len() + bs.len())
        .map_err(|_| Error::new_outofmemory())?;

    buf.append_slice(as_.as_bytes())
        .map_err(|_| Error::new_outofmemory())?;
    buf.append_slice(bs.as_bytes())
        .map_err(|_| Error::new_outofmemory())?;

    unsafe { RString::new(buf.as_str_unchecked()).map(|v| v.cast_value()) }
}

fn stirng__len(this: &RValue, _args: &[RValue]) -> Result<RValue, Error> {
    unsafe {
        let s = this.expect_cast::<RString>(string_type())?;
        RInt::new(s.len() as isize).map(|v| v.cast_value())
    }
}

pub fn string_repr(s: &Ref<RString>) -> Result<Ref<RString>, Error> {
    fn append(buf: &mut Array<u8>, s: &str) -> Result<(), Error> {
        buf.append_slice(s.as_bytes())
            .map_err(|_| Error::new_outofmemory())?;
        Ok(())
    }

    let mut buf = Array::new(allocator());
    let mut char_buf = [0; 8];

    buf.reserve(s.len() + 2)
        .map_err(|_| Error::new_outofmemory())?;

    append(&mut buf, "\"")?;

    for c in s.as_str().chars() {
        match c {
            '\t' => append(&mut buf, "\\t")?,
            '\r' => append(&mut buf, "\\r")?,
            '\n' => append(&mut buf, "\\n")?,
            '\"' => append(&mut buf, "\\\"")?,
            '\\' => append(&mut buf, "\\\\")?,
            c => append(&mut buf, c.encode_utf8(&mut char_buf))?,
        }
    }

    append(&mut buf, "\"")?;

    unsafe { RString::new(buf.as_str_unchecked()) }
}
