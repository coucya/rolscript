use core::fmt::{Debug, Formatter, Result as FmtResult};
use core::hash::{Hash, Hasher};
use core::mem::MaybeUninit;
use core::ptr::addr_of_mut;
use core::ptr::NonNull;

use crate::collections::ListNodeBase;
use crate::collections::ToListNode;

use crate::op::*;
use crate::runtime::*;

use crate::error::*;
use crate::runtime_error_fmt;

use crate::number::*;
use crate::option::ROption;
use crate::string::*;
use crate::type_::*;

use crate::builtin::*;

use crate::nonnull_of;

#[repr(C)]
pub struct GcHeader {
    _list_node: ListNodeBase,
    _type: MaybeUninit<Ref<RType>>,
    _block_size: usize,
    _ref_count: usize,
    _mark: bool,
}

impl ToListNode for NonNull<GcHeader> {
    fn to_base(self) -> NonNull<ListNodeBase> {
        unsafe { core::mem::transmute(self) }
    }
}

#[allow(dead_code)]
impl GcHeader {
    pub(crate) unsafe fn init(
        mut ptr: NonNull<Self>,
        block_size: usize,
        type_: Option<Ref<RType>>,
    ) {
        ListNodeBase::init(nonnull_of!(ptr.as_mut()._list_node));

        if let Some(tp) = type_ {
            addr_of_mut!(ptr.as_mut()._type).write(MaybeUninit::new(tp));
        } else {
            addr_of_mut!(ptr.as_mut()._type).write(MaybeUninit::uninit());
        }

        addr_of_mut!(ptr.as_mut()._block_size).write(block_size);
        addr_of_mut!(ptr.as_mut()._ref_count).write(0);
        addr_of_mut!(ptr.as_mut()._mark).write(false);
    }

    pub(crate) fn drop_head(mut ptr: NonNull<Self>) {
        unsafe {
            addr_of_mut!(ptr.as_mut()._type).drop_in_place();
        }
    }

    pub(crate) fn mark(&self) -> bool {
        self._mark
    }
    pub(crate) fn set_mark(&mut self, m: bool) {
        self._mark = m;
    }

    pub(crate) fn block_size(&self) -> usize {
        self._block_size
    }
    pub(crate) fn ref_count(&self) -> usize {
        self._ref_count
    }

    pub(crate) fn inc_ref(&mut self) {
        self._ref_count += 1;
    }
    pub(crate) fn dec_ref(&mut self) {
        self._ref_count -= 1;
    }

    pub fn is_type(&self, tp: &Ref<RType>) -> bool {
        unsafe { Ref::ptr_eq(self._type.assume_init_ref(), tp) }
    }

    pub fn get_type(&self) -> &Ref<RType> {
        unsafe { self._type.assume_init_ref() }
    }
}

pub(crate) struct TypeUninitRef<T: ?Sized>(NonNull<T>);

#[allow(dead_code)]
impl<T: ?Sized> TypeUninitRef<T> {
    pub(crate) unsafe fn from_raw(ptr: NonNull<T>) -> Self {
        ptr.cast::<GcHeader>().as_mut().inc_ref();
        Self(ptr)
    }

    #[inline]
    pub(crate) unsafe fn force_into(self) -> Ref<T> {
        let n = Ref(self.0);
        core::mem::forget(self);
        n
    }

    #[inline]
    pub(crate) unsafe fn cast<U>(self) -> TypeUninitRef<U> {
        let ptr = self.0;
        core::mem::forget(self);
        TypeUninitRef(ptr.cast())
    }

    #[inline]
    pub(crate) unsafe fn cast_ref<U>(&self) -> &TypeUninitRef<U> {
        core::mem::transmute(self)
    }
    #[inline]
    pub(crate) unsafe fn cast_mut<U>(&mut self) -> &mut TypeUninitRef<U> {
        core::mem::transmute(self)
    }

    pub(crate) unsafe fn init_type(self, tp: Ref<RType>) -> Ref<T> {
        unsafe {
            self.0.cast::<GcHeader>().as_mut()._type.write(tp);
            self.force_into()
        }
    }

    pub(crate) fn as_nonnull_ptr(&self) -> NonNull<T> {
        self.0
    }

    pub(crate) fn as_ref(&self) -> &T {
        unsafe { self.0.as_ref() }
    }
    pub(crate) fn as_mut(&mut self) -> &mut T {
        unsafe { self.0.as_mut() }
    }
}

impl<T: ?Sized> Clone for TypeUninitRef<T> {
    fn clone(&self) -> Self {
        unsafe { TypeUninitRef::from_raw(self.0) }
    }
}

impl<T: ?Sized> Drop for TypeUninitRef<T> {
    fn drop(&mut self) {
        unsafe {
            self.cast_mut::<GcHeader>().dec_ref();
        }
    }
}

impl<T: ?Sized> core::ops::Deref for TypeUninitRef<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}
impl<T: ?Sized> core::ops::DerefMut for TypeUninitRef<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut()
    }
}

#[repr(transparent)]
pub struct Ref<T: ?Sized>(NonNull<T>);

#[allow(dead_code)]
impl<T: ?Sized> Ref<T> {
    pub unsafe fn from_raw(ptr: NonNull<T>) -> Self {
        ptr.cast::<GcHeader>().as_mut().inc_ref();
        Self(ptr)
    }

    pub(crate) fn cast_value_ref(&self) -> &Ref<GcHeader> {
        unsafe { core::mem::transmute(self) }
    }
    pub(crate) fn cast_value_mut(&mut self) -> &mut Ref<GcHeader> {
        unsafe { core::mem::transmute(self) }
    }

    pub fn cast_value(&self) -> Ref<GcHeader> {
        unsafe { Ref::from_raw(self.0.cast()) }
    }

    #[inline]
    pub unsafe fn cast<U>(self) -> Ref<U> {
        let ptr = self.0;
        core::mem::forget(self);
        Ref(ptr.cast())
    }

    #[inline]
    pub unsafe fn cast_ref<U>(&self) -> &Ref<U> {
        core::mem::transmute(self)
    }

    #[inline]
    pub unsafe fn cast_mut<U>(&mut self) -> &mut Ref<U> {
        core::mem::transmute(self)
    }

    #[inline]
    pub unsafe fn expect_cast<U>(&self, tp: &Ref<RType>) -> Result<Ref<U>, Error> {
        if self.is_type(tp) {
            Ok(self.clone().cast())
        } else {
            Err(Error::new_type(tp.clone(), self.get_type().clone()))
        }
    }

    #[inline]
    pub fn ptr_eq(a: &Self, b: &Self) -> bool {
        a.as_ptr() == b.as_ptr()
    }

    pub(crate) fn as_ptr(&self) -> *const T {
        unsafe { self.0.as_ref() as *const T }
    }
    pub(crate) fn as_mut_ptr(&mut self) -> *mut T {
        unsafe { self.0.as_mut() as *mut T }
    }

    pub(crate) fn as_nonnull_ptr(&self) -> NonNull<T> {
        self.0
    }

    pub(crate) fn as_ref(&self) -> &T {
        unsafe { self.0.as_ref() }
    }
    pub(crate) fn as_mut(&mut self) -> &mut T {
        unsafe { self.0.as_mut() }
    }
}

impl<T: ?Sized> Ref<T> {
    pub fn is_type(&self, tp: &Ref<RType>) -> bool {
        unsafe { Ref::ptr_eq(self.cast_value_ref()._type.assume_init_ref(), tp) }
    }

    pub fn get_type(&self) -> &Ref<RType> {
        unsafe { self.cast_value_ref()._type.assume_init_ref() }
    }
}

unsafe impl<T: ?Sized> Send for Ref<T> {}
unsafe impl<T: ?Sized> Sync for Ref<T> {}

impl<T: ?Sized> Drop for Ref<T> {
    fn drop(&mut self) {
        self.cast_value_mut().dec_ref();
    }
}

impl<T: ?Sized> Clone for Ref<T> {
    fn clone(&self) -> Self {
        unsafe { Ref::from_raw(self.0) }
    }
}

impl<T: ?Sized> core::ops::Deref for Ref<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}
impl<T: ?Sized> core::ops::DerefMut for Ref<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut()
    }
}

pub type RValue = Ref<GcHeader>;

impl Hash for RValue {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_usize(self.as_ptr() as usize);
    }
}

impl Debug for RValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_fmt(format_args!(
            "<{} at 0x{:X}>",
            self.get_type().name().as_str(),
            self.as_ptr() as usize
        ))
    }
}

#[inline]
pub fn value_new(tp: &Ref<RType>, args: &[RValue]) -> Result<RValue, Error> {
    if let Some(new_) = tp._new {
        new_(tp, args)
    } else {
        Err(runtime_error_fmt!(
            "\"{}\" not support constructors",
            tp.name().as_str()
        ))
    }
}

#[inline]
pub fn value_destory(value: &RValue) -> Result<(), Error> {
    let tp = value.get_type();
    if let Some(destory) = tp._destory {
        destory(value)
    } else {
        Ok(())
    }
}

#[inline]
pub fn value_get_method(value: &RValue, name: &Ref<RString>) -> Result<RValue, Error> {
    let tp = value.get_type();
    if let Some(v) = tp.get_attr(name).cloned() {
        Ok(v.clone())
    } else {
        Err(runtime_error_fmt!(
            "{:?} has no method \"{}\"",
            value,
            name.as_str()
        ))
    }
}

#[inline]
pub fn value_get_method_v(value: &RValue, name: &RValue) -> Result<RValue, Error> {
    if name.is_type(&string_type()) {
        let tp = value.get_type();
        let name = unsafe { name.cast_ref::<RString>() };
        if let Some(v) = tp.get_attr(name).cloned() {
            Ok(v.clone())
        } else {
            Err(runtime_error_fmt!(
                "{:?} has no method \"{}\"",
                value,
                name.as_str()
            ))
        }
    } else {
        Err(runtime_error_fmt!("method name must be is string"))
    }
}

#[inline]
pub fn value_get_method_try(
    value: &RValue,
    name: &Ref<RString>,
) -> Result<Option<RValue>, Error> {
    let tp = value.get_type();
    Ok(tp.get_attr(name).cloned())
}

#[inline]
pub fn value_get_method_try_v(value: &RValue, name: &RValue) -> Result<Option<RValue>, Error> {
    let tp = value.get_type();
    if name.is_type(&string_type()) {
        let name = unsafe { name.cast_ref::<RString>() };
        Ok(tp.get_attr(name).cloned())
    } else {
        Err(runtime_error_fmt!("method name must be is string"))
    }
}

#[inline]
pub fn value_call_method(
    value: &RValue,
    name: &Ref<RString>,
    args: &[RValue],
) -> Result<RValue, Error> {
    let tp = value.get_type();
    let method = tp.get_attr(name);
    if let Some(method) = method {
        value_call_with_this(&method, value, args)
    } else {
        Err(runtime_error_fmt!(
            "{:?} has no method \"{}\"",
            value,
            name.as_str()
        ))
    }
}

#[inline]
pub fn value_call_method_v(
    value: &RValue,
    name: &RValue,
    args: &[RValue],
) -> Result<RValue, Error> {
    if name.is_type(&string_type()) {
        let tp = value.get_type();
        let name = unsafe { name.cast_ref::<RString>() };
        if let Some(method) = tp.get_attr(name) {
            value_call_with_this(&method, value, args)
        } else {
            Err(runtime_error_fmt!(
                "{:?} has no method \"{}\"",
                value,
                name.as_str()
            ))
        }
    } else {
        Err(runtime_error_fmt!("method name must be is string"))
    }
}

#[inline]
pub fn value_call_attr(
    value: &RValue,
    name: &Ref<RString>,
    args: &[RValue],
) -> Result<RValue, Error> {
    let func = value_get_attr(value, name)?;
    value_call_with_this(&func, value, args)
}

#[inline]
pub fn value_call_attr_v(value: &RValue, name: &RValue, args: &[RValue]) -> Result<RValue, Error> {
    let func = value_get_attr_v(value, name)?;
    value_call_with_this(&func, value, args)
}

#[inline]
pub fn value_get_attr(value: &RValue, name: &Ref<RString>) -> Result<RValue, Error> {
    let tp = value.get_type();
    if let Some(get_attr) = tp._get_attr {
        get_attr(value, name)
    } else {
        Err(runtime_error_fmt!("{:?} cannot get attribute", value))
    }
}

#[inline]
pub fn value_set_attr(
    value: &RValue,
    name: &Ref<RString>,
    attr_value: &RValue,
) -> Result<(), Error> {
    let tp = value.get_type();
    if let Some(set_attr) = tp._set_attr {
        set_attr(value, name, attr_value)
    } else {
        Err(runtime_error_fmt!("{:?} cannot set attribute", value))
    }
}

#[inline]
pub fn value_get_attr_v(value: &RValue, name: &RValue) -> Result<RValue, Error> {
    let tp = value.get_type();
    if let Some(get_attr) = tp._get_attr {
        if name.is_type(string_type()) {
            unsafe { get_attr(value, name.cast_ref::<RString>()) }
        } else {
            Err(runtime_error_fmt!("attribute name must be string"))
        }
    } else {
        Err(runtime_error_fmt!("{:?} cannot get attribute", value))
    }
}

#[inline]
pub fn value_set_attr_v(value: &RValue, name: &RValue, attr_value: &RValue) -> Result<(), Error> {
    let tp = value.get_type();
    if let Some(set_attr) = tp._set_attr {
        if name.is_type(string_type()) {
            unsafe { set_attr(value, name.cast_ref::<RString>(), attr_value) }
        } else {
            Err(runtime_error_fmt!("attr name must be string"))
        }
    } else {
        Err(runtime_error_fmt!("{:?} cannot get attr", value))
    }
}

#[inline]
pub fn value_get_item(value: &RValue, idx: &RValue) -> Result<RValue, Error> {
    let tp = value.get_type();
    if let Some(get_item) = tp._get_item {
        get_item(value, idx)
    } else {
        Err(runtime_error_fmt!(
            "\"{}\" is not subscriptable",
            tp.name().as_str()
        ))
    }
}

#[inline]
pub fn value_set_item(value: &RValue, idx: &RValue, item: &RValue) -> Result<(), Error> {
    let tp = value.get_type();
    if let Some(set_item) = tp._set_item {
        set_item(value, idx, item)
    } else {
        Err(runtime_error_fmt!(
            "\"{}\" does not support item assignment",
            tp.name().as_str()
        ))
    }
}

#[inline]
pub(crate) fn _value_call_raw(
    callee: &RValue,
    this: &RValue,
    args: &[RValue],
) -> Result<RValue, Error> {
    let tp = callee.get_type();
    if let Some(call) = tp._call {
        call(callee, this, args)
    } else {
        Err(runtime_error_fmt!(
            "\"{}\" is not callable",
            tp.name().as_str()
        ))
    }
}

#[inline]
pub fn value_call(callee: &RValue, args: &[RValue]) -> Result<RValue, Error> {
    let null_v = null().cast_value();
    runtime()._call(callee, &null_v, args)
}

#[inline]
pub fn value_call_with_this(
    callee: &RValue,
    this: &RValue,
    args: &[RValue],
) -> Result<RValue, Error> {
    runtime()._call(callee, this, args)
}

#[inline]
pub fn value_binary_op(op: ArithOp, value: &RValue, other: &RValue) -> Result<RValue, Error> {
    let tp = value.get_type();
    if let Some(op) = tp._arith[op as usize] {
        op(value, other)
    } else {
        Err(runtime_error_fmt!(
            "unsupported operand type for '{:?}': \"{}\" and \"{}\"",
            op,
            tp.name().as_str(),
            other.get_type().name().as_str()
        ))
    }
}

#[inline]
pub fn value_unary_op(op: UnaryOp, value: &RValue) -> Result<RValue, Error> {
    let tp = value.get_type();
    if let Some(op) = tp._unary[op as usize] {
        op(value)
    } else {
        Err(runtime_error_fmt!(
            "unsupported operand type for '{:?}': \"{}\"",
            op,
            tp.name().as_str(),
        ))
    }
}

#[inline]
pub fn value_eq(value: &RValue, other: &RValue) -> Result<bool, Error> {
    let tp = value.get_type();
    if let Some(eq) = tp._eq {
        eq(value, other)
    } else {
        Err(runtime_error_fmt!(
            "unsupported operand type for \"==\": \"{}\" and \"{}\"",
            tp.name().as_str(),
            other.get_type().name().as_str()
        ))
    }
}

#[inline]
pub fn value_cmp(value: &RValue, other: &RValue) -> Result<Int, Error> {
    let tp = value.get_type();
    if let Some(cmp) = tp._cmp {
        cmp(value, other)
    } else {
        Err(runtime_error_fmt!(
            "\"{}\" and \"{}\" do not support comparison operation",
            tp.name().as_str(),
            other.get_type().name().as_str()
        ))
    }
}

#[inline]
pub fn value_str(value: &RValue) -> Result<Ref<RString>, Error> {
    let tp = value.get_type();
    if let Some(to_string) = tp._str {
        to_string(value)
    } else {
        RString::format(format_args!("{:?}", value))
    }
}

#[inline]
pub fn value_repr(value: &RValue) -> Result<Ref<RString>, Error> {
    let tp = value.get_type();
    unsafe {
        if value.is_type(string_type()) {
            string_repr(value.cast_ref::<RString>())
        } else {
            if let Some(to_string) = tp._str {
                to_string(value)
            } else {
                RString::format(format_args!("{:?}", value))
            }
        }
    }
}

#[inline]
pub fn value_hash(value: &RValue) -> Result<Int, Error> {
    let tp = value.get_type();
    if let Some(hash) = tp._hash {
        hash(value)
    } else {
        Err(runtime_error_fmt!(
            "\"{}\" is not hashable",
            tp.name().as_str()
        ))
    }
}

#[inline]
pub fn value_iter(value: &RValue) -> Result<RValue, Error> {
    let tp = value.get_type();
    if let Some(iter) = tp._iter {
        iter(value)
    } else {
        Err(runtime_error_fmt!(
            "\"{}\" is not iterable",
            tp.name().as_str()
        ))
    }
}

#[inline]
pub fn value_next(value: &RValue) -> Result<Ref<ROption>, Error> {
    let tp = value.get_type();
    if let Some(next) = tp._next {
        next(value)
    } else {
        Err(runtime_error_fmt!(
            "\"{}\" is not iterator",
            tp.name().as_str()
        ))
    }
}

#[inline]
pub fn value_visit(visitor: &mut dyn Visitor, value: &RValue) {
    let tp = value.get_type();

    visitor.visit_value(tp.cast_value_ref());

    if let Some(visit) = tp._visit {
        visit(visitor, value.as_nonnull_ptr())
    }
}

#[inline]
pub fn value_visit_ptr(visitor: &mut dyn Visitor, value_ptr: NonNull<GcHeader>) {
    unsafe {
        let tp = value_ptr.as_ref().get_type();

        visitor.visit_value(tp.cast_value_ref());

        if let Some(visit) = tp._visit {
            visit(visitor, value_ptr)
        }
    }
}

pub fn default_value_eq(value: &RValue, other: &RValue) -> Result<bool, Error> {
    Ok(other.is_type(value.get_type()) && Ref::ptr_eq(value, other))
}

pub fn default_value_hash(value: &RValue) -> Result<Int, Error> {
    Ok(value.as_ptr() as usize as Int)
}

pub fn default_value_str(value: &RValue) -> Result<Ref<RString>, Error> {
    RString::format(format_args!(
        "<{} at 0x{:x}>",
        value.get_type().name().as_str(),
        value.as_ptr() as usize
    ))
}
