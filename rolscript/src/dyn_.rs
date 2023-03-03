#![allow(non_snake_case)]

use core::mem::size_of;
use core::ptr::addr_of_mut;
use core::ptr::NonNull;

use crate::runtime::*;

use crate::op::*;

use crate::error::*;
use crate::runtime_error_fmt;

use crate::number::*;
use crate::string::*;
use crate::type_::*;
use crate::value::*;

use crate::builtin::*;

use crate::util::StringMap;

/// 脚本中动态创建的类型所产生的实例对象用该结构体表示。
#[repr(C)]
struct RDyn {
    _header: GcHeader,
    _attr: StringMap<RValue>,
}

impl RDyn {
    unsafe fn init(mut ptr: NonNull<Self>) {
        addr_of_mut!(ptr.as_mut()._attr).write(StringMap::new(allocator()));
    }

    pub(crate) fn new(tp: &Ref<RType>) -> Result<Ref<Self>, Error> {
        unsafe {
            let v = new_gc_obj(size_of::<RDyn>(), tp.clone())?.cast::<Self>();
            Self::init(v.as_nonnull_ptr());
            Ok(v)
        }
    }
}

pub fn type_new_dyn(type_name: &Ref<RString>) -> Result<Ref<RType>, Error> {
    let mut tp = RType::new(type_name.clone())?;
    tp.set_dyn(true);

    tp.with_visit(_dyn__visit);

    tp.with_new(_dyn__new);
    tp.with_destory(_dyn__destory);

    tp.with_get_attr(_dyn__get_attr);
    tp.with_set_attr(_dyn__set_attr);
    tp.with_get_item(_dyn__get_item);
    tp.with_set_item(_dyn__set_item);

    tp.with_call(_dyn__call);
    tp.with_eq(_dyn__eq);
    tp.with_cmp(_dyn__cmp);
    tp.with_str(_dyn__to_string);
    tp.with_hash(_dyn__hash);

    tp.with_arith(ArithOp::Add, _dyn__add);
    tp.with_arith(ArithOp::Sub, _dyn__sub);
    tp.with_arith(ArithOp::Mul, _dyn__mod);
    tp.with_arith(ArithOp::Div, _dyn__div);
    tp.with_arith(ArithOp::IDiv, _dyn__idiv);
    tp.with_arith(ArithOp::Mod, _dyn__mod);
    tp.with_arith(ArithOp::Pow, _dyn__pow);
    tp.with_arith(ArithOp::And, _dyn__add);
    tp.with_arith(ArithOp::Or, _dyn__or);
    tp.with_arith(ArithOp::BitAnd, _dyn__add);
    tp.with_arith(ArithOp::BitOr, _dyn__bitor);
    tp.with_arith(ArithOp::BitXor, _dyn__bitxor);
    tp.with_arith(ArithOp::Shl, _dyn__shl);
    tp.with_arith(ArithOp::Shr, _dyn__shr);

    tp.with_unary(UnaryOp::Not, _dyn__not);
    tp.with_unary(UnaryOp::BitNot, _dyn__bitnot);

    Ok(tp)
}

fn _dyn__visit(visitor: &mut dyn Visitor, value_ptr: NonNull<GcHeader>) {
    unsafe {
        let dyn_ = value_ptr.cast::<RDyn>();
        let dyn_ref = dyn_.as_ref();
        for (k, v) in dyn_ref._attr.iter() {
            visitor.visit_value(k.cast_value_ref());
            visitor.visit_value(v);
        }
    }
}

fn _dyn__new(tp: &Ref<RType>, args: &[RValue]) -> Result<RValue, Error> {
    if let Some(func) = &tp._new_dyn {
        let this = RDyn::new(tp)?.cast_value();
        value_call_with_this(func, &this, args)?;
        Ok(this)
    } else {
        let tp_name = tp.name();
        Err(runtime_error_fmt!(
            "\"{}\" does not support constructors",
            tp_name.as_str()
        ))
    }
}

fn _dyn__destory(this: &RValue) -> Result<(), Error> {
    let tp = this.get_type();
    if tp._isdyn {
        if let Some(func) = &tp._destory_dyn {
            value_call_with_this(func, this, &[])?;
        }

        unsafe {
            let mut v = this.clone().cast::<RDyn>();
            addr_of_mut!(v.as_mut()._attr).drop_in_place();
        }
    }
    Ok(())
}

fn _dyn__get_attr(value: &RValue, name: &Ref<RString>) -> Result<RValue, Error> {
    let tp = value.get_type();
    if tp._isdyn {
        if let Some(func) = &tp._get_attr_dyn {
            return value_call_with_this(func, value, &[name.cast_value()]);
        } else {
            let rdyn = unsafe { value.cast_ref::<RDyn>() };
            if let Some(v) = rdyn._attr.get(name) {
                return Ok(v.clone());
            }
        }
    }
    let vs = value_str(value)?;
    Err(runtime_error_fmt!(
        "{} has no attribute \"{}\"",
        vs.as_str(),
        name.as_str()
    ))
}

fn _dyn__set_attr(value: &RValue, name: &Ref<RString>, attr_value: &RValue) -> Result<(), Error> {
    let tp = value.get_type();
    if tp._isdyn {
        if let Some(func) = &tp._set_attr_dyn {
            return value_call_with_this(func, value, &[name.cast_value(), attr_value.clone()])
                .map(|_| ());
        } else {
            let mut rdyn = unsafe { value.clone().cast::<RDyn>() };
            rdyn._attr.insert(name.clone(), attr_value.clone())?;
            return Ok(());
        }
    }
    let vs = value_str(value)?;
    Err(runtime_error_fmt!(
        "{} unable to set attribute",
        vs.as_str()
    ))
}

fn _dyn__get_item(value: &RValue, index: &RValue) -> Result<RValue, Error> {
    let tp = value.get_type();
    if tp._isdyn {
        if let Some(func) = &tp._get_item_dyn {
            return value_call_with_this(func, value, &[index.clone()]);
        }
    }
    Err(runtime_error_fmt!(
        "{:?} has no subscript \"{:?}\"",
        value,
        index
    ))
}

fn _dyn__set_item(value: &RValue, index: &RValue, item_value: &RValue) -> Result<(), Error> {
    let tp = value.get_type();
    if tp._isdyn {
        if let Some(func) = &tp._set_attr_dyn {
            return value_call_with_this(func, value, &[index.clone(), item_value.clone()])
                .map(|_| ());
        }
    }

    Err(runtime_error_fmt!(
        "{:?} unable to set subscript {:?}",
        value,
        index
    ))
}

fn _dyn__call(callee: &RValue, _this: &RValue, args: &[RValue]) -> Result<RValue, Error> {
    let tp = callee.get_type();
    if tp._isdyn {
        if let Some(func) = &tp._call_dyn {
            return value_call_with_this(func, callee, args);
        }
    }

    Err(runtime_error_fmt!("{:?} not callable", callee))
}

fn _dyn__eq(value: &RValue, other: &RValue) -> Result<bool, Error> {
    let tp = value.get_type();
    if tp._isdyn {
        if let Some(func) = &tp._eq_dyn {
            let res = value_call_with_this(func, value, &[other.clone()])?;
            return Ok(value_to_bool(&res));
        }
    }

    Err(runtime_error_fmt!(
        "unsupported operand type for \"==\": \"{}\" and \"{}\"",
        tp.name().as_str(),
        other.get_type().name().as_str()
    ))
}

fn _dyn__cmp(value: &RValue, other: &RValue) -> Result<Int, Error> {
    let tp = value.get_type();
    if tp._isdyn {
        if let Some(func) = &tp._cmp_dyn {
            let res = value_call_with_this(func, value, &[other.clone()])?;
            if res.is_type(int_type()) {
                unsafe {
                    return Ok(res.cast_ref::<RInt>().as_number());
                }
            } else {
                return Err(runtime_error_fmt!(
                    "the return value of the comparison operator of \"{}\" is invalid,",
                    tp.name().as_str(),
                ));
            }
        }
    }

    Err(runtime_error_fmt!(
        "\"{}\" and \"{}\" do not support comparison operation",
        tp.name().as_str(),
        other.get_type().name().as_str()
    ))
}

fn _dyn__to_string(value: &RValue) -> Result<Ref<RString>, Error> {
    let tp = value.get_type();
    if tp._isdyn {
        if let Some(func) = &tp._str_dyn {
            let res = value_call_with_this(func, value, &[])?;
            if res.is_type(string_type()) {
                unsafe {
                    return Ok(res.cast::<RString>());
                }
            } else {
                return Err(runtime_error_fmt!(
                    "the return value of the \"to_string\" function of \"{:?}\" is invalid,",
                    value
                ));
            }
        }
    }
    RString::format(format_args!("{:?}", value))
}

fn _dyn__hash(value: &RValue) -> Result<Int, Error> {
    let tp = value.get_type();
    if tp._isdyn {
        if let Some(func) = &tp._hash_dyn {
            let res = value_call_with_this(func, value, &[])?;
            if res.is_type(int_type()) {
                unsafe {
                    return Ok(res.cast::<RInt>().as_number());
                }
            } else {
                return Err(runtime_error_fmt!(
                    "the return value of the \"hash\" function of \"{:?}\" is invalid,",
                    value
                ));
            }
        }
    }
    Err(runtime_error_fmt!(
        "\"{}\" is not hashable",
        tp.name().as_str()
    ))
}

fn _dyn__add(value: &RValue, other: &RValue) -> Result<RValue, Error> {
    let tp = value.get_type();
    let op = ArithOp::Add;
    if tp._isdyn {
        if let Some(func) = &tp._arith_dyn[op as usize] {
            return value_call_with_this(func, value, &[other.clone()]);
        }
    }
    Err(runtime_error_fmt!(
        "unsupported operand type for '{:?}': \"{}\" and \"{}\"",
        op,
        tp.name().as_str(),
        other.get_type().name().as_str()
    ))
}

fn _dyn__sub(value: &RValue, other: &RValue) -> Result<RValue, Error> {
    let tp = value.get_type();
    let op = ArithOp::Sub;
    if tp._isdyn {
        if let Some(func) = &tp._arith_dyn[op as usize] {
            return value_call_with_this(func, value, &[other.clone()]);
        }
    }
    Err(runtime_error_fmt!(
        "unsupported operand type for '{:?}': \"{}\" and \"{}\"",
        op,
        tp.name().as_str(),
        other.get_type().name().as_str()
    ))
}

fn _dyn__mul(value: &RValue, other: &RValue) -> Result<RValue, Error> {
    let tp = value.get_type();
    let op = ArithOp::Mul;
    if tp._isdyn {
        if let Some(func) = &tp._arith_dyn[op as usize] {
            return value_call_with_this(func, value, &[other.clone()]);
        }
    }
    Err(runtime_error_fmt!(
        "unsupported operand type for '{:?}': \"{}\" and \"{}\"",
        op,
        tp.name().as_str(),
        other.get_type().name().as_str()
    ))
}

fn _dyn__div(value: &RValue, other: &RValue) -> Result<RValue, Error> {
    let tp = value.get_type();
    let op = ArithOp::Div;
    if tp._isdyn {
        if let Some(func) = &tp._arith_dyn[op as usize] {
            return value_call_with_this(func, value, &[other.clone()]);
        }
    }
    Err(runtime_error_fmt!(
        "unsupported operand type for '{:?}': \"{}\" and \"{}\"",
        op,
        tp.name().as_str(),
        other.get_type().name().as_str()
    ))
}

fn _dyn__idiv(value: &RValue, other: &RValue) -> Result<RValue, Error> {
    let tp = value.get_type();
    let op = ArithOp::IDiv;
    if tp._isdyn {
        if let Some(func) = &tp._arith_dyn[op as usize] {
            return value_call_with_this(func, value, &[other.clone()]);
        }
    }
    Err(runtime_error_fmt!(
        "unsupported operand type for '{:?}': \"{}\" and \"{}\"",
        op,
        tp.name().as_str(),
        other.get_type().name().as_str()
    ))
}

fn _dyn__mod(value: &RValue, other: &RValue) -> Result<RValue, Error> {
    let tp = value.get_type();
    let op = ArithOp::Mod;
    if tp._isdyn {
        if let Some(func) = &tp._arith_dyn[op as usize] {
            return value_call_with_this(func, value, &[other.clone()]);
        }
    }
    Err(runtime_error_fmt!(
        "unsupported operand type for '{:?}': \"{}\" and \"{}\"",
        op,
        tp.name().as_str(),
        other.get_type().name().as_str()
    ))
}

fn _dyn__pow(value: &RValue, other: &RValue) -> Result<RValue, Error> {
    let tp = value.get_type();
    let op = ArithOp::Pow;
    if tp._isdyn {
        if let Some(func) = &tp._arith_dyn[op as usize] {
            return value_call_with_this(func, value, &[other.clone()]);
        }
    }
    Err(runtime_error_fmt!(
        "unsupported operand type for '{:?}': \"{}\" and \"{}\"",
        op,
        tp.name().as_str(),
        other.get_type().name().as_str()
    ))
}

fn _dyn__and(value: &RValue, other: &RValue) -> Result<RValue, Error> {
    let tp = value.get_type();
    let op = ArithOp::And;
    if tp._isdyn {
        if let Some(func) = &tp._arith_dyn[op as usize] {
            return value_call_with_this(func, value, &[other.clone()]);
        }
    }
    Err(runtime_error_fmt!(
        "unsupported operand type for '{:?}': \"{}\" and \"{}\"",
        op,
        tp.name().as_str(),
        other.get_type().name().as_str()
    ))
}

fn _dyn__or(value: &RValue, other: &RValue) -> Result<RValue, Error> {
    let tp = value.get_type();
    let op = ArithOp::Or;
    if tp._isdyn {
        if let Some(func) = &tp._arith_dyn[op as usize] {
            return value_call_with_this(func, value, &[other.clone()]);
        }
    }
    Err(runtime_error_fmt!(
        "unsupported operand type for '{:?}': \"{}\" and \"{}\"",
        op,
        tp.name().as_str(),
        other.get_type().name().as_str()
    ))
}

fn _dyn__bitand(value: &RValue, other: &RValue) -> Result<RValue, Error> {
    let tp = value.get_type();
    let op = ArithOp::BitAnd;
    if tp._isdyn {
        if let Some(func) = &tp._arith_dyn[op as usize] {
            return value_call_with_this(func, value, &[other.clone()]);
        }
    }
    Err(runtime_error_fmt!(
        "unsupported operand type for '{:?}': \"{}\" and \"{}\"",
        op,
        tp.name().as_str(),
        other.get_type().name().as_str()
    ))
}

fn _dyn__bitor(value: &RValue, other: &RValue) -> Result<RValue, Error> {
    let tp = value.get_type();
    let op = ArithOp::BitOr;
    if tp._isdyn {
        if let Some(func) = &tp._arith_dyn[op as usize] {
            return value_call_with_this(func, value, &[other.clone()]);
        }
    }
    Err(runtime_error_fmt!(
        "unsupported operand type for '{:?}': \"{}\" and \"{}\"",
        op,
        tp.name().as_str(),
        other.get_type().name().as_str()
    ))
}

fn _dyn__bitxor(value: &RValue, other: &RValue) -> Result<RValue, Error> {
    let tp = value.get_type();
    let op = ArithOp::BitXor;
    if tp._isdyn {
        if let Some(func) = &tp._arith_dyn[op as usize] {
            return value_call_with_this(func, value, &[other.clone()]);
        }
    }
    Err(runtime_error_fmt!(
        "unsupported operand type for '{:?}': \"{}\" and \"{}\"",
        op,
        tp.name().as_str(),
        other.get_type().name().as_str()
    ))
}

fn _dyn__shl(value: &RValue, other: &RValue) -> Result<RValue, Error> {
    let tp = value.get_type();
    let op = ArithOp::Shl;
    if tp._isdyn {
        if let Some(func) = &tp._arith_dyn[op as usize] {
            return value_call_with_this(func, value, &[other.clone()]);
        }
    }
    Err(runtime_error_fmt!(
        "unsupported operand type for '{:?}': \"{}\" and \"{}\"",
        op,
        tp.name().as_str(),
        other.get_type().name().as_str()
    ))
}

fn _dyn__shr(value: &RValue, other: &RValue) -> Result<RValue, Error> {
    let tp = value.get_type();
    let op = ArithOp::Shr;
    if tp._isdyn {
        if let Some(func) = &tp._arith_dyn[op as usize] {
            return value_call_with_this(func, value, &[other.clone()]);
        }
    }
    Err(runtime_error_fmt!(
        "unsupported operand type for '{:?}': \"{}\" and \"{}\"",
        op,
        tp.name().as_str(),
        other.get_type().name().as_str()
    ))
}

fn _dyn__not(value: &RValue) -> Result<RValue, Error> {
    let tp = value.get_type();
    let op = UnaryOp::Not;
    if tp._isdyn {
        if let Some(func) = &tp._unary_dyn[op as usize] {
            return value_call_with_this(func, value, &[]);
        }
    }
    Err(runtime_error_fmt!(
        "unsupported operand type for '{:?}': \"{}\"",
        op,
        tp.name().as_str(),
    ))
}

fn _dyn__bitnot(value: &RValue) -> Result<RValue, Error> {
    let tp = value.get_type();
    let op = UnaryOp::BitNot;
    if tp._isdyn {
        if let Some(func) = &tp._unary_dyn[op as usize] {
            return value_call_with_this(func, value, &[]);
        }
    }
    Err(runtime_error_fmt!(
        "unsupported operand type for '{:?}': \"{}\"",
        op,
        tp.name().as_str(),
    ))
}
