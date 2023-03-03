#![allow(non_snake_case)]

use core::fmt::Write;
use core::mem::size_of;

use crate::collections::FixedStrBuf;

use crate::runtime::*;

use crate::op::*;

use crate::error::*;
use crate::runtime_error_fmt;

use crate::string::*;
use crate::type_::*;
use crate::value::*;

use crate::builtin::*;

pub type Int = isize;
#[cfg(target_pointer_width = "32")]
pub type Float = f32;
#[cfg(target_pointer_width = "64")]
pub type Float = f64;

#[repr(C)]
pub struct RNull {
    _header: GcHeader,
}

#[repr(C)]
pub struct RBool {
    _header: GcHeader,
    _bool: bool,
}

#[repr(C)]
pub struct RInt {
    _header: GcHeader,
    _number: Int,
}

#[repr(C)]
pub struct RFloat {
    _header: GcHeader,
    _number: Float,
}

impl RBool {
    #[inline]
    pub fn new(b: bool) -> Result<Ref<RBool>, Error> {
        if b {
            Ok(true_().clone())
        } else {
            Ok(false_().clone())
        }
    }

    #[inline]
    pub fn as_bool(&self) -> bool {
        self._bool
    }
}

impl RInt {
    #[inline]
    pub fn new(n: Int) -> Result<Ref<RInt>, Error> {
        runtime().integer_pool_get(n)
    }

    #[inline]
    pub fn as_number(&self) -> Int {
        self._number
    }
}

impl RFloat {
    #[inline]
    pub fn new(n: Float) -> Result<Ref<RFloat>, Error> {
        let tp = float_type().clone();
        unsafe {
            let mut v = new_gc_obj(size_of::<Self>(), tp)?.cast::<Self>();
            v._number = n;
            Ok(v)
        }
    }

    #[inline]
    pub fn as_number(&self) -> Float {
        self._number
    }
}

pub(crate) fn _new_null_value() -> Result<Ref<RNull>, Error> {
    unsafe {
        let tp = null_type().clone();
        let v = new_gc_obj(size_of::<RNull>(), tp)?.cast::<RNull>();
        Ok(v)
    }
}

pub(crate) fn _new_bool_value(b: bool) -> Result<Ref<RBool>, Error> {
    unsafe {
        let tp = bool_type().clone();
        let mut v = new_gc_obj(size_of::<RBool>(), tp)?.cast::<RBool>();
        v._bool = b;
        Ok(v)
    }
}

pub(crate) fn _new_int_value(n: Int) -> Result<Ref<RInt>, Error> {
    unsafe {
        let tp = int_type().clone();
        let mut v = new_gc_obj(size_of::<RInt>(), tp)?.cast::<RInt>();
        v._number = n;
        Ok(v)
    }
}

pub(crate) fn _new_float_value(n: Float) -> Result<Ref<RFloat>, Error> {
    unsafe {
        let tp = float_type().clone();
        let mut v = new_gc_obj(size_of::<RFloat>(), tp)?.cast::<RFloat>();
        v._number = n;
        Ok(v)
    }
}

pub(crate) fn _init_type_null(mut tp: Ref<RType>) -> Result<(), Error> {
    tp.with_eq(default_value_eq);
    tp.with_hash(|_| Ok(0));
    tp.with_str(null__to_string);

    Ok(())
}

pub(crate) fn _init_type_bool(mut tp: Ref<RType>) -> Result<(), Error> {
    tp.with_arith(ArithOp::And, bool__and);
    tp.with_arith(ArithOp::Or, bool__or);
    tp.with_unary(UnaryOp::Not, bool__not);

    tp.with_new(bool__new);
    tp.with_eq(default_value_eq);
    tp.with_hash(bool__hash);
    tp.with_str(bool__to_string);

    Ok(())
}

pub(crate) fn _init_type_int(mut tp: Ref<RType>) -> Result<(), Error> {
    tp.with_arith(ArithOp::Add, int__add);
    tp.with_arith(ArithOp::Sub, int__sub);
    tp.with_arith(ArithOp::Mul, int__mul);
    tp.with_arith(ArithOp::Div, int__div);
    tp.with_arith(ArithOp::IDiv, int__idiv);
    tp.with_arith(ArithOp::Mod, int__mod);
    tp.with_arith(ArithOp::Pow, int__pow);
    tp.with_arith(ArithOp::BitAnd, int__bitand);
    tp.with_arith(ArithOp::BitOr, int__bitor);
    tp.with_arith(ArithOp::BitXor, int__bitxor);
    tp.with_arith(ArithOp::Shl, int__shl);
    tp.with_arith(ArithOp::Shr, int__shr);

    tp.with_unary(UnaryOp::BitNot, int__bitnot);

    tp.with_new(int__new);

    tp.with_eq(int__eq);
    tp.with_cmp(int__cmp);

    tp.with_hash(int__hash);
    tp.with_str(int__to_string);

    Ok(())
}

pub(crate) fn _init_type_float(mut tp: Ref<RType>) -> Result<(), Error> {
    tp.with_arith(ArithOp::Add, float__add);
    tp.with_arith(ArithOp::Sub, float__sub);
    tp.with_arith(ArithOp::Mul, float__mul);
    tp.with_arith(ArithOp::Div, float__div);
    tp.with_arith(ArithOp::IDiv, float__idiv);
    tp.with_arith(ArithOp::Mod, float__mod);
    tp.with_arith(ArithOp::Pow, float__pow);

    tp.with_new(float__new);
    tp.with_cmp(float__cmp);
    tp.with_eq(float__eq);
    tp.with_hash(float__hash);
    tp.with_str(float__to_string);

    Ok(())
}

use crate::util::expect_arg1;

fn unsupported_operand_error(op_name: &str, a: &RValue, b: &RValue) -> Error {
    runtime_error_fmt!(
        "unsupported operand type for '{}': \"{}\" and \"{}\"",
        op_name,
        a.get_type().name().as_str(),
        b.get_type().name().as_str()
    )
}

fn null__to_string(_: &RValue) -> Result<Ref<RString>, Error> {
    RString::new("null")
}

fn bool__new(_tp: &Ref<RType>, args: &[RValue]) -> Result<RValue, Error> {
    let arg = expect_arg1(args)?;
    if arg.is_type(null_type()) {
        Ok(false_().cast_value())
    } else if arg.is_type(bool_type()) {
        Ok(arg)
    } else {
        Ok(true_().cast_value())
    }
}
fn bool__and(instance: &RValue, right: &RValue) -> Result<RValue, Error> {
    let l = unsafe { instance.expect_cast::<RBool>(&bool_type())? };
    let r = unsafe { right.expect_cast::<RBool>(&bool_type())? };
    if l.as_bool() && r.as_bool() {
        Ok(false_().cast_value())
    } else {
        Ok(true_().cast_value())
    }
}
fn bool__or(instance: &RValue, right: &RValue) -> Result<RValue, Error> {
    let l = unsafe { instance.expect_cast::<RBool>(&bool_type())? };
    let r = unsafe { right.expect_cast::<RBool>(&bool_type())? };
    if l.as_bool() || r.as_bool() {
        Ok(false_().cast_value())
    } else {
        Ok(true_().cast_value())
    }
}
fn bool__not(instance: &RValue) -> Result<RValue, Error> {
    let l = unsafe { instance.expect_cast::<RBool>(&bool_type())? };
    if l.as_bool() {
        Ok(false_().cast_value())
    } else {
        Ok(true_().cast_value())
    }
}
fn bool__hash(v: &RValue) -> Result<Int, Error> {
    unsafe { Ok(v.expect_cast::<RBool>(bool_type())?.as_bool() as Int) }
}
fn bool__to_string(instance: &RValue) -> Result<Ref<RString>, Error> {
    let l = unsafe { instance.expect_cast::<RBool>(bool_type())? };
    RString::new(if l.as_bool() { "true" } else { "false" })
}

fn int__new(_tp: &Ref<RType>, args: &[RValue]) -> Result<RValue, Error> {
    let arg = expect_arg1(args)?;
    if arg.is_type(bool_type()) {
        let b = unsafe { arg.cast_ref::<RBool>() };
        if b.as_bool() {
            Ok(RInt::new(1)?.cast_value())
        } else {
            Ok(RInt::new(0)?.cast_value())
        }
    } else if arg.is_type(int_type()) {
        Ok(arg)
    } else if arg.is_type(float_type()) {
        let f = unsafe { arg.cast_ref::<RFloat>() };
        Ok(RInt::new(f.as_number().trunc() as Int)?.cast_value())
    } else if arg.is_type(string_type()) {
        let s = unsafe { arg.cast_ref::<RString>() };
        if let Ok(n) = s.as_str().parse::<isize>() {
            Ok(RInt::new(n as Int)?.cast_value())
        } else if let Ok(n) = s.as_str().parse::<f64>() {
            Ok(RInt::new(n.trunc() as Int)?.cast_value())
        } else {
            Err(runtime_error_fmt!(
                "\"{}\" cannot be converted to Int",
                s.as_str()
            ))
        }
    } else {
        let arg_s = value_str(&arg)?;
        Err(runtime_error_fmt!(
            "{} cannot be converted to Int",
            arg_s.as_str()
        ))
    }
}

fn int__hash(v: &RValue) -> Result<Int, Error> {
    unsafe { Ok(v.expect_cast::<RInt>(int_type())?.as_number()) }
}

fn int__add(instance: &RValue, right: &RValue) -> Result<RValue, Error> {
    let l = unsafe { instance.expect_cast::<RInt>(int_type())?.as_number() };
    if right.is_type(int_type()) {
        let r = unsafe { right.cast_ref::<RInt>().as_number() };
        let res = l.wrapping_add(r);
        Ok(RInt::new(res)?.cast_value())
    } else if right.is_type(float_type()) {
        let r = unsafe { right.cast_ref::<RFloat>().as_number() };
        let res = l as Float + r;
        Ok(RFloat::new(res)?.cast_value())
    } else {
        Err(unsupported_operand_error("+", instance, right))
    }
}

fn int__sub(instance: &RValue, right: &RValue) -> Result<RValue, Error> {
    let l = unsafe { instance.expect_cast::<RInt>(int_type())?.as_number() };
    if right.is_type(int_type()) {
        let r = unsafe { right.cast_ref::<RInt>().as_number() };
        let res = l.wrapping_sub(r);
        Ok(RInt::new(res)?.cast_value())
    } else if right.is_type(float_type()) {
        let r = unsafe { right.cast_ref::<RFloat>().as_number() };
        let res = l as Float - r;
        Ok(RFloat::new(res)?.cast_value())
    } else {
        Err(unsupported_operand_error("-", instance, right))
    }
}

fn int__mul(instance: &RValue, right: &RValue) -> Result<RValue, Error> {
    let l = unsafe { instance.expect_cast::<RInt>(int_type())?.as_number() };
    if right.is_type(int_type()) {
        let r = unsafe { right.cast_ref::<RInt>().as_number() };
        let res = l.wrapping_mul(r);
        Ok(RInt::new(res)?.cast_value())
    } else if right.is_type(float_type()) {
        let r = unsafe { right.cast_ref::<RFloat>().as_number() };
        let res = l as Float * r;
        Ok(RFloat::new(res)?.cast_value())
    } else {
        Err(unsupported_operand_error("*", instance, right))
    }
}

fn int__div(instance: &RValue, right: &RValue) -> Result<RValue, Error> {
    let l = unsafe { instance.expect_cast::<RInt>(int_type())?.as_number() };
    if right.is_type(int_type()) {
        let r = unsafe { right.cast_ref::<RInt>().as_number() };
        let v = (l as Float) / (r as Float);
        if (v as Int as Float) == v {
            Ok(RInt::new(v as Int)?.cast_value())
        } else {
            Ok(RFloat::new(v)?.cast_value())
        }
    } else if right.is_type(float_type()) {
        let r = unsafe { right.cast_ref::<RFloat>().as_number() };
        let res = l as Float / r;
        Ok(RFloat::new(res)?.cast_value())
    } else {
        Err(unsupported_operand_error("/", instance, right))
    }
}

fn int__idiv(instance: &RValue, right: &RValue) -> Result<RValue, Error> {
    let l = unsafe { instance.expect_cast::<RInt>(int_type())?.as_number() };
    if right.is_type(int_type()) {
        let r = unsafe { right.cast_ref::<RInt>().as_number() };
        let res = l.div_euclid(r);
        Ok(RInt::new(res)?.cast_value())
    } else if right.is_type(float_type()) {
        let r = unsafe { right.cast_ref::<RFloat>().as_number() };
        let res = (l as Float).div_euclid(r);
        Ok(RFloat::new(res)?.cast_value())
    } else {
        Err(unsupported_operand_error("//", instance, right))
    }
}

fn int__mod(instance: &RValue, right: &RValue) -> Result<RValue, Error> {
    let l = unsafe { instance.expect_cast::<RInt>(int_type())?.as_number() };
    if right.is_type(int_type()) {
        let r = unsafe { right.cast_ref::<RInt>().as_number() };
        let res = l.rem_euclid(r);
        Ok(RInt::new(res)?.cast_value())
    } else if right.is_type(float_type()) {
        let r = unsafe { right.cast_ref::<RFloat>().as_number() };
        let res = (l as Float).rem_euclid(r);
        Ok(RFloat::new(res)?.cast_value())
    } else {
        Err(unsupported_operand_error("%", instance, right))
    }
}

fn int__pow(instance: &RValue, right: &RValue) -> Result<RValue, Error> {
    let l = unsafe { instance.expect_cast::<RInt>(int_type())?.as_number() };
    if right.is_type(int_type()) {
        let r = unsafe { right.cast_ref::<RInt>().as_number() };
        if r >= 0 {
            let res = l.pow(r as u32);
            Ok(RInt::new(res)?.cast_value())
        } else {
            let res = (l as Float).powi(r as i32);
            Ok(RFloat::new(res)?.cast_value())
        }
    } else if right.is_type(float_type()) {
        let r = unsafe { right.cast_ref::<RFloat>().as_number() };
        let res = (l as Float).powf(r);
        Ok(RFloat::new(res)?.cast_value())
    } else {
        Err(unsupported_operand_error("**", instance, right))
    }
}

fn int__bitand(instance: &RValue, right: &RValue) -> Result<RValue, Error> {
    let l = unsafe { instance.expect_cast::<RInt>(int_type())?.as_number() };
    if right.is_type(int_type()) {
        let r = unsafe { right.cast_ref::<RInt>().as_number() };
        Ok(RInt::new(l & r)?.cast_value())
    } else {
        Err(unsupported_operand_error("&", instance, right))
    }
}

fn int__bitor(instance: &RValue, right: &RValue) -> Result<RValue, Error> {
    let l = unsafe { instance.expect_cast::<RInt>(int_type())?.as_number() };
    if right.is_type(int_type()) {
        let r = unsafe { right.cast_ref::<RInt>().as_number() };
        Ok(RInt::new(l | r)?.cast_value())
    } else {
        Err(unsupported_operand_error("|", instance, right))
    }
}

fn int__bitxor(instance: &RValue, right: &RValue) -> Result<RValue, Error> {
    let l = unsafe { instance.expect_cast::<RInt>(int_type())?.as_number() };
    if right.is_type(int_type()) {
        let r = unsafe { right.cast_ref::<RInt>().as_number() };
        Ok(RInt::new(l ^ r)?.cast_value())
    } else {
        Err(unsupported_operand_error("^", instance, right))
    }
}

fn int__bitnot(instance: &RValue) -> Result<RValue, Error> {
    let l = unsafe { instance.expect_cast::<RInt>(int_type())?.as_number() };
    Ok(RInt::new(!l)?.cast_value())
}

fn int__shl(instance: &RValue, right: &RValue) -> Result<RValue, Error> {
    let l = unsafe { instance.expect_cast::<RInt>(int_type())?.as_number() };
    if right.is_type(int_type()) {
        let r = unsafe { right.cast_ref::<RInt>().as_number() };
        if r >= 0 {
            Ok(RInt::new(l.wrapping_shl(r as u32))?.cast_value())
        } else {
            Err(runtime_error_fmt!("negative shift count"))
        }
    } else {
        Err(unsupported_operand_error("<<", instance, right))
    }
}

fn int__shr(instance: &RValue, right: &RValue) -> Result<RValue, Error> {
    let l = unsafe { instance.expect_cast::<RInt>(int_type())?.as_number() };
    if right.is_type(int_type()) {
        let r = unsafe { right.cast_ref::<RInt>().as_number() };
        if r >= 0 {
            Ok(RInt::new(l.wrapping_shr(r as u32))?.cast_value())
        } else {
            Err(runtime_error_fmt!("negative shift count"))
        }
    } else {
        Err(unsupported_operand_error(">>", instance, right))
    }
}

fn int__eq(instance: &RValue, right: &RValue) -> Result<bool, Error> {
    let l = unsafe { instance.expect_cast::<RInt>(int_type())?.as_number() };
    if right.is_type(int_type()) {
        let r = unsafe { right.cast_ref::<RInt>().as_number() };
        Ok(l == r)
    } else if right.is_type(float_type()) {
        let r = unsafe { right.cast_ref::<RFloat>().as_number() };
        Ok(l as Float == r)
    } else {
        Ok(false)
    }
}

fn int__cmp(instance: &RValue, right: &RValue) -> Result<Int, Error> {
    let l = unsafe { instance.expect_cast::<RInt>(int_type())?.as_number() };
    if right.is_type(int_type()) {
        let r = unsafe { right.cast_ref::<RInt>().as_number() };
        Ok(l.cmp(&r) as Int)
    } else if right.is_type(float_type()) {
        let r = unsafe { right.cast_ref::<RFloat>().as_number() };
        if let Some(n) = (l as Float).partial_cmp(&r) {
            Ok(n as Int)
        } else {
            let ls = value_str(instance)?;
            let rs = value_str(right)?;
            Err(runtime_error_fmt!(
                "{} and {} cannot be compared",
                ls.as_str(),
                rs.as_str(),
            ))
        }
    } else {
        Err(unsupported_operand_error("<cmp>", instance, right))
    }
}

#[allow(unused_must_use)]
fn int__to_string(instance: &RValue) -> Result<Ref<RString>, Error> {
    let l = unsafe { instance.expect_cast::<RInt>(int_type())?.as_number() };
    let mut buf = FixedStrBuf::<32>::new();
    write!(&mut buf, "{}", l);

    RString::new(buf.as_str())
}

fn float__new(_tp: &Ref<RType>, args: &[RValue]) -> Result<RValue, Error> {
    let arg = expect_arg1(args)?;
    if arg.is_type(bool_type()) {
        let b = unsafe { arg.cast_ref::<RBool>() };
        if b.as_bool() {
            Ok(RFloat::new(1.0)?.cast_value())
        } else {
            Ok(RFloat::new(0.0)?.cast_value())
        }
    } else if arg.is_type(int_type()) {
        let i = unsafe { arg.cast_ref::<RInt>() };
        Ok(RFloat::new(i.as_number() as f64 as Float)?.cast_value())
    } else if arg.is_type(float_type()) {
        Ok(arg)
    } else if arg.is_type(string_type()) {
        let s = unsafe { arg.cast_ref::<RString>() };
        if let Ok(n) = s.as_str().parse::<f64>() {
            Ok(RFloat::new(n as Float)?.cast_value())
        } else {
            Err(runtime_error_fmt!(
                "\"{}\" cannot be converted to Float",
                s.as_str()
            ))
        }
    } else {
        let arg_s = value_str(&arg)?;
        Err(runtime_error_fmt!(
            "{} cannot be converted to Float",
            arg_s.as_str()
        ))
    }
}

fn float__hash(v: &RValue) -> Result<Int, Error> {
    unsafe {
        let n = v.expect_cast::<RFloat>(float_type())?.as_number();
        Ok(core::mem::transmute(n))
    }
}

fn float__add(instance: &RValue, right: &RValue) -> Result<RValue, Error> {
    let l = unsafe { instance.expect_cast::<RFloat>(int_type())?.as_number() };
    if right.is_type(int_type()) {
        let r = unsafe { right.cast_ref::<RInt>().as_number() };
        let res = l + r as Float;
        Ok(RFloat::new(res)?.cast_value())
    } else if right.is_type(float_type()) {
        let r = unsafe { right.cast_ref::<RFloat>().as_number() };
        let res = l + r;
        Ok(RFloat::new(res)?.cast_value())
    } else {
        Err(unsupported_operand_error("+", instance, right))
    }
}

fn float__sub(instance: &RValue, right: &RValue) -> Result<RValue, Error> {
    let l = unsafe { instance.expect_cast::<RFloat>(int_type())?.as_number() };
    if right.is_type(int_type()) {
        let r = unsafe { right.cast_ref::<RInt>().as_number() };
        let res = l - r as Float;
        Ok(RFloat::new(res)?.cast_value())
    } else if right.is_type(float_type()) {
        let r = unsafe { right.cast_ref::<RFloat>().as_number() };
        let res = l - r;
        Ok(RFloat::new(res)?.cast_value())
    } else {
        Err(unsupported_operand_error("-", instance, right))
    }
}

fn float__mul(instance: &RValue, right: &RValue) -> Result<RValue, Error> {
    let l = unsafe { instance.expect_cast::<RFloat>(int_type())?.as_number() };
    if right.is_type(int_type()) {
        let r = unsafe { right.cast_ref::<RInt>().as_number() };
        let res = l * r as Float;
        Ok(RFloat::new(res)?.cast_value())
    } else if right.is_type(float_type()) {
        let r = unsafe { right.cast_ref::<RFloat>().as_number() };
        let res = l * r;
        Ok(RFloat::new(res)?.cast_value())
    } else {
        Err(unsupported_operand_error("*", instance, right))
    }
}

fn float__div(instance: &RValue, right: &RValue) -> Result<RValue, Error> {
    let l = unsafe { instance.expect_cast::<RFloat>(int_type())?.as_number() };
    if right.is_type(int_type()) {
        let r = unsafe { right.cast_ref::<RInt>().as_number() };
        let res = l / r as Float;
        Ok(RFloat::new(res)?.cast_value())
    } else if right.is_type(float_type()) {
        let r = unsafe { right.cast_ref::<RFloat>().as_number() };
        let res = l / r;
        Ok(RFloat::new(res)?.cast_value())
    } else {
        Err(unsupported_operand_error("/", instance, right))
    }
}

fn float__idiv(instance: &RValue, right: &RValue) -> Result<RValue, Error> {
    let l = unsafe { instance.expect_cast::<RFloat>(int_type())?.as_number() };
    if right.is_type(int_type()) {
        let r = unsafe { right.cast_ref::<RInt>().as_number() };
        let res = l.div_euclid(r as Float);
        Ok(RFloat::new(res)?.cast_value())
    } else if right.is_type(float_type()) {
        let r = unsafe { right.cast_ref::<RFloat>().as_number() };
        let res = l.div_euclid(r);
        Ok(RFloat::new(res)?.cast_value())
    } else {
        Err(unsupported_operand_error("//", instance, right))
    }
}

fn float__mod(instance: &RValue, right: &RValue) -> Result<RValue, Error> {
    let l = unsafe { instance.expect_cast::<RFloat>(int_type())?.as_number() };
    if right.is_type(int_type()) {
        let r = unsafe { right.cast_ref::<RInt>().as_number() };
        let res = l.rem_euclid(r as Float);
        Ok(RFloat::new(res)?.cast_value())
    } else if right.is_type(float_type()) {
        let r = unsafe { right.cast_ref::<RFloat>().as_number() };
        let res = l.rem_euclid(r);
        Ok(RFloat::new(res)?.cast_value())
    } else {
        Err(unsupported_operand_error("%", instance, right))
    }
}

fn float__pow(instance: &RValue, right: &RValue) -> Result<RValue, Error> {
    let l = unsafe { instance.expect_cast::<RFloat>(int_type())?.as_number() };
    if right.is_type(int_type()) {
        let r = unsafe { right.cast_ref::<RInt>().as_number() };
        let res = l.powi(r as i32);
        Ok(RFloat::new(res)?.cast_value())
    } else if right.is_type(float_type()) {
        let r = unsafe { right.cast_ref::<RFloat>().as_number() };
        let res = l.powf(r);
        Ok(RFloat::new(res)?.cast_value())
    } else {
        Err(unsupported_operand_error("**", instance, right))
    }
}

fn float__eq(instance: &RValue, right: &RValue) -> Result<bool, Error> {
    let l = unsafe { instance.expect_cast::<RFloat>(int_type())?.as_number() };
    if right.is_type(int_type()) {
        let r = unsafe { right.cast_ref::<RInt>().as_number() };
        Ok(l == (r as Float))
    } else if right.is_type(float_type()) {
        let r = unsafe { right.cast_ref::<RFloat>().as_number() };
        Ok(l == r)
    } else {
        Ok(false)
    }
}

fn float__cmp(instance: &RValue, right: &RValue) -> Result<Int, Error> {
    let l = unsafe { instance.expect_cast::<RFloat>(int_type())?.as_number() };
    let cmp_res = if right.is_type(int_type()) {
        let r = unsafe { right.cast_ref::<RInt>().as_number() };
        l.partial_cmp(&(r as Float))
    } else if right.is_type(float_type()) {
        let r = unsafe { right.cast_ref::<RFloat>().as_number() };
        l.partial_cmp(&r)
    } else {
        return Err(unsupported_operand_error("<cmp>", instance, right));
    };

    if let Some(n) = cmp_res {
        Ok(n as Int)
    } else {
        let ls = value_str(instance)?;
        let rs = value_str(right)?;
        Err(runtime_error_fmt!(
            "{} and {} cannot be compared",
            ls.as_str(),
            rs.as_str(),
        ))
    }
}

#[allow(unused_must_use)]
fn float__to_string(instance: &RValue) -> Result<Ref<RString>, Error> {
    let l = unsafe { instance.expect_cast::<RFloat>(float_type())?.as_number() };
    let mut buf = FixedStrBuf::<64>::new();
    write!(&mut buf, "{}", l);

    RString::new(buf.as_str())
}

/// 当value为null和false时返回false，其他的返回true。
pub fn value_to_bool(value: &RValue) -> bool {
    if value.is_type(bool_type()) {
        unsafe { value.cast_ref::<RBool>().as_bool() }
    } else if value.is_type(null_type()) {
        return false;
    } else {
        return true;
    }
}
