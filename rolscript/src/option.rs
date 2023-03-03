use core::mem::size_of;
use core::ptr::addr_of_mut;
use core::ptr::NonNull;

use crate::runtime::*;

use crate::error::*;

use crate::function::*;
use crate::type_::*;
use crate::value::*;

use crate::builtin::*;

use crate::util::expect_arg1;

#[repr(C)]
pub struct ROption {
    _header: GcHeader,
    _value: Option<RValue>,
}

impl ROption {
    #[inline]
    unsafe fn init(mut ptr: NonNull<Self>, value: Option<RValue>) {
        addr_of_mut!(ptr.as_mut()._value).write(value);
    }

    #[inline]
    pub(crate) fn new_(value: Option<RValue>) -> Result<Ref<Self>, Error> {
        unsafe {
            let ptr = new_gc_obj(size_of::<Self>(), option_type().clone())?.cast::<Self>();
            Self::init(ptr.as_nonnull_ptr(), value);
            Ok(ptr)
        }
    }

    #[inline]
    pub fn new(value: Option<RValue>) -> Result<Ref<Self>, Error> {
        if let Some(v) = value {
            Self::new_(Some(v))
        } else {
            Ok(super::option::none().clone())
        }
    }

    #[inline]
    pub fn some(value: RValue) -> Result<Ref<Self>, Error> {
        Self::new_(Some(value))
    }

    #[inline]
    pub fn none() -> Result<Ref<Self>, Error> {
        Ok(super::option::none().clone())
    }

    pub fn is_some(&self) -> bool {
        self._value.is_some()
    }
    pub fn is_none(&self) -> bool {
        self._value.is_none()
    }

    pub fn value(&self) -> &Option<RValue> {
        &self._value
    }
}

pub(crate) fn _init_type_option(mut tp: Ref<RType>) -> Result<(), Error> {
    tp.with_eq(option__eq);
    tp.with_hash(default_value_hash);
    tp.with_str(default_value_str);

    tp.add_method_str_light("is_some", option__is_some)?;
    tp.add_method_str_light("is_none", option__is_none)?;
    tp.add_method_str_light("value", option__value)?;

    let some_func = RFunction::from_rust_func(|_, args| {
        let arg = expect_arg1(args)?;
        let v = ROption::some(arg)?.cast_value();
        Ok(v)
    })?;
    let none_func = RFunction::from_rust_func(|_, _| {
        let v = ROption::none()?.cast_value();
        Ok(v)
    })?;
    tp.set_attr_str("some", some_func.cast_value())?;
    tp.set_attr_str("none", none_func.cast_value())?;

    Ok(())
}

#[allow(non_snake_case)]
fn option__eq(value: &RValue, other: &RValue) -> Result<bool, Error> {
    let op = unsafe { value.expect_cast::<ROption>(option_type())? };
    let res = if other.is_type(option_type()) {
        let other_op = unsafe { other.expect_cast::<ROption>(option_type())? };
        match (&op._value, &other_op._value) {
            (Some(a), Some(b)) => value_eq(a, b)?,
            (None, None) => true,
            _ => false,
        }
    } else {
        false
    };
    Ok(res)
}

#[allow(non_snake_case)]
fn option__is_some(this: &RValue, _args: &[RValue]) -> Result<RValue, Error> {
    let op = unsafe { this.expect_cast::<ROption>(option_type())? };
    if op.is_some() {
        Ok(true_().cast_value())
    } else {
        Ok(false_().cast_value())
    }
}

#[allow(non_snake_case)]
fn option__is_none(this: &RValue, _args: &[RValue]) -> Result<RValue, Error> {
    let op = unsafe { this.expect_cast::<ROption>(option_type())? };
    if op.is_none() {
        Ok(true_().cast_value())
    } else {
        Ok(false_().cast_value())
    }
}

#[allow(non_snake_case)]
fn option__value(this: &RValue, _args: &[RValue]) -> Result<RValue, Error> {
    let op = unsafe { this.expect_cast::<ROption>(option_type())? };
    if let Some(v) = op.value() {
        Ok(v.clone())
    } else {
        Ok(null().cast_value())
    }
}
