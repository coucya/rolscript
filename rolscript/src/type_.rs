use core::array::from_fn;
use core::fmt::{Debug, Formatter, Result as FmtResult};
use core::mem::size_of;
use core::ptr::addr_of_mut;
use core::ptr::NonNull;

use crate::alloc::Allocator;

use crate::op::*;
use crate::runtime::*;

use crate::error::*;
use crate::runtime_error_fmt;

use crate::value::*;

use crate::function::RFunction;
use crate::function::RRustFunction;
use crate::number::*;
use crate::option::ROption;
use crate::string::*;

use crate::builtin::*;

use crate::util::StringMap;

pub type NewFunc = fn(&Ref<RType>, &[RValue]) -> Result<RValue, Error>;
pub type DestoryFunc = fn(&RValue) -> Result<(), Error>;
/// (value, name);                 // value.name
pub type GetAttrFunc = fn(&RValue, &Ref<RString>) -> Result<RValue, Error>;
/// (value, name, attr_value);     // value.name = attr_value
pub type SetAttrFunc = fn(&RValue, &Ref<RString>, &RValue) -> Result<(), Error>;
/// (value, index);                // value\[index\]
pub type GetItemFunc = fn(&RValue, &RValue) -> Result<RValue, Error>;
/// (value, index, item);          // value\[index\] = item
pub type SetItemFunc = fn(&RValue, &RValue, &RValue) -> Result<(), Error>;
/// (callee, this, args) -> value;
pub type CallFunc = fn(&RValue, &RValue, &[RValue]) -> Result<RValue, Error>;
/// (a, b) -> value;               // a op b
pub type ArithFunc = fn(&RValue, &RValue) -> Result<RValue, Error>;
/// (a) -> value;                  // op a
pub type UnaryFunc = fn(&RValue) -> Result<RValue, Error>;
/// (a, b) -> bool;                // a == b
pub type EqFunc = fn(&RValue, &RValue) -> Result<bool, Error>;
/// (a, b) -> int;                 // cmp(a, b) -> lt(-1) | Eq(0) | Gt(1)
pub type CmpFunc = fn(&RValue, &RValue) -> Result<Int, Error>;
pub type StrFunc = fn(&RValue) -> Result<Ref<RString>, Error>;
pub type HashFunc = fn(&RValue) -> Result<Int, Error>;
pub type IterFunc = fn(&RValue) -> Result<RValue, Error>;
pub type NextFunc = fn(&RValue) -> Result<Ref<ROption>, Error>;
pub type VisitFunc = fn(&mut dyn Visitor, NonNull<GcHeader>);

#[repr(C)]
pub struct RType {
    _header: GcHeader,

    pub(crate) _isdyn: bool,
    _name: Ref<RString>,
    _attrs: StringMap<RValue>,

    // 用于遍历对象所引用的值。
    // 主要用于垃圾回收。
    pub(crate) _visit: Option<VisitFunc>,

    pub(crate) _new: Option<NewFunc>,
    pub(crate) _destory: Option<DestoryFunc>,
    pub(crate) _get_attr: Option<GetAttrFunc>,
    pub(crate) _set_attr: Option<SetAttrFunc>,
    pub(crate) _get_item: Option<GetItemFunc>,
    pub(crate) _set_item: Option<SetItemFunc>,
    pub(crate) _call: Option<CallFunc>,
    pub(crate) _eq: Option<EqFunc>,
    pub(crate) _cmp: Option<CmpFunc>,
    pub(crate) _str: Option<StrFunc>,
    pub(crate) _hash: Option<HashFunc>,
    pub(crate) _iter: Option<IterFunc>,
    pub(crate) _next: Option<NextFunc>,

    pub(crate) _arith: [Option<ArithFunc>; ARITH_OP_COUNT],
    pub(crate) _unary: [Option<UnaryFunc>; UNARY_OP_COUNT],

    // 以下字段给在脚本中动态创建的类型使用。
    pub(crate) _new_dyn: Option<RValue>,
    pub(crate) _destory_dyn: Option<RValue>,
    pub(crate) _get_attr_dyn: Option<RValue>,
    pub(crate) _set_attr_dyn: Option<RValue>,
    pub(crate) _get_item_dyn: Option<RValue>,
    pub(crate) _set_item_dyn: Option<RValue>,
    pub(crate) _call_dyn: Option<RValue>,
    pub(crate) _eq_dyn: Option<RValue>,
    pub(crate) _cmp_dyn: Option<RValue>,
    pub(crate) _str_dyn: Option<RValue>,
    pub(crate) _hash_dyn: Option<RValue>,
    pub(crate) _iter_dyn: Option<RValue>,
    pub(crate) _next_dyn: Option<RValue>,

    pub(crate) _arith_dyn: [Option<RValue>; ARITH_OP_COUNT],
    pub(crate) _unary_dyn: [Option<RValue>; UNARY_OP_COUNT],
}

impl RType {
    pub(crate) fn need_size() -> usize {
        size_of::<Self>()
    }

    pub(crate) unsafe fn init(
        allocator: &'static dyn Allocator,
        mut ptr: NonNull<Self>,
        name: Ref<RString>,
    ) {
        let r = ptr.as_mut();
        addr_of_mut!(r._isdyn).write(false);
        addr_of_mut!(r._name).write(name);
        addr_of_mut!(r._attrs).write(StringMap::new(allocator));

        addr_of_mut!(r._visit).write(None);

        addr_of_mut!(r._new).write(None);
        addr_of_mut!(r._destory).write(None);
        addr_of_mut!(r._get_attr).write(None);
        addr_of_mut!(r._set_attr).write(None);
        addr_of_mut!(r._get_item).write(None);
        addr_of_mut!(r._set_item).write(None);
        addr_of_mut!(r._call).write(None);
        addr_of_mut!(r._eq).write(None);
        addr_of_mut!(r._cmp).write(None);
        addr_of_mut!(r._str).write(None);
        addr_of_mut!(r._hash).write(None);
        addr_of_mut!(r._iter).write(None);
        addr_of_mut!(r._next).write(None);

        addr_of_mut!(r._arith).write([None; ARITH_OP_COUNT]);
        addr_of_mut!(r._unary).write([None; UNARY_OP_COUNT]);

        addr_of_mut!(r._new_dyn).write(None);
        addr_of_mut!(r._destory_dyn).write(None);
        addr_of_mut!(r._get_attr_dyn).write(None);
        addr_of_mut!(r._set_attr_dyn).write(None);
        addr_of_mut!(r._get_item_dyn).write(None);
        addr_of_mut!(r._set_item_dyn).write(None);
        addr_of_mut!(r._call_dyn).write(None);
        addr_of_mut!(r._eq_dyn).write(None);
        addr_of_mut!(r._cmp_dyn).write(None);
        addr_of_mut!(r._str_dyn).write(None);
        addr_of_mut!(r._hash_dyn).write(None);
        addr_of_mut!(r._iter_dyn).write(None);
        addr_of_mut!(r._next_dyn).write(None);

        addr_of_mut!(r._arith_dyn).write(from_fn(|_| None));
        addr_of_mut!(r._unary_dyn).write(from_fn(|_| None));
    }

    unsafe fn _drop(&mut self) {
        addr_of_mut!(self._name).drop_in_place();
        addr_of_mut!(self._attrs).drop_in_place();

        addr_of_mut!(self._new_dyn).drop_in_place();
        addr_of_mut!(self._destory_dyn).drop_in_place();
        addr_of_mut!(self._get_attr_dyn).drop_in_place();
        addr_of_mut!(self._set_attr_dyn).drop_in_place();
        addr_of_mut!(self._get_item_dyn).drop_in_place();
        addr_of_mut!(self._set_item_dyn).drop_in_place();
        addr_of_mut!(self._call_dyn).drop_in_place();
        addr_of_mut!(self._eq_dyn).drop_in_place();
        addr_of_mut!(self._cmp_dyn).drop_in_place();
        addr_of_mut!(self._str_dyn).drop_in_place();
        addr_of_mut!(self._hash_dyn).drop_in_place();
        addr_of_mut!(self._iter_dyn).drop_in_place();
        addr_of_mut!(self._next_dyn).drop_in_place();

        addr_of_mut!(self._arith_dyn).drop_in_place();
        addr_of_mut!(self._unary_dyn).drop_in_place();
    }

    pub(crate) fn set_dyn(&mut self, is_dyn: bool) {
        self._isdyn = is_dyn;
    }

    pub fn new(name: Ref<RString>) -> Result<Ref<Self>, Error> {
        unsafe {
            let tptp = type_type().clone();
            let v = new_gc_obj(size_of::<Self>(), tptp)?.cast::<Self>();
            Self::init(allocator(), v.as_nonnull_ptr(), name);
            Ok(v)
        }
    }

    pub fn new_with_str(name: &str) -> Result<Ref<Self>, Error> {
        unsafe {
            let name = RString::new(name)?;
            let tptp = type_type().clone();
            let v = new_gc_obj(size_of::<Self>(), tptp)?.cast::<Self>();
            Self::init(allocator(), v.as_nonnull_ptr(), name);
            Ok(v)
        }
    }

    pub fn name(&self) -> &Ref<RString> {
        &self._name
    }
}

// native
impl RType {
    pub fn with_visit(&mut self, visit: VisitFunc) {
        self._visit = Some(visit);
    }

    pub fn with_new(&mut self, new_func: NewFunc) {
        self._new = Some(new_func);
    }

    pub fn with_destory(&mut self, destory: DestoryFunc) {
        self._destory = Some(destory);
    }

    pub fn with_get_attr(&mut self, get_attr_func: GetAttrFunc) {
        self._get_attr = Some(get_attr_func);
    }

    pub fn with_set_attr(&mut self, set_attr_func: SetAttrFunc) {
        self._set_attr = Some(set_attr_func);
    }

    pub fn with_get_item(&mut self, get_item_func: GetItemFunc) {
        self._get_item = Some(get_item_func);
    }

    pub fn with_set_item(&mut self, set_item_func: SetItemFunc) {
        self._set_item = Some(set_item_func);
    }

    pub fn with_call(&mut self, call_func: CallFunc) {
        self._call = Some(call_func);
    }

    pub fn with_arith(&mut self, op: ArithOp, op_func: ArithFunc) {
        self._arith[op as usize] = Some(op_func);
    }

    pub fn with_unary(&mut self, op: UnaryOp, op_func: UnaryFunc) {
        self._unary[op as usize] = Some(op_func);
    }

    pub fn with_cmp(&mut self, cmp: CmpFunc) {
        self._cmp = Some(cmp)
    }

    pub fn with_eq(&mut self, eq: EqFunc) {
        self._eq = Some(eq)
    }

    pub fn with_str(&mut self, str_func: StrFunc) {
        self._str = Some(str_func)
    }

    pub fn with_repr(&mut self, repr_func: StrFunc) {
        self._str = Some(repr_func)
    }

    pub fn with_hash(&mut self, hash_func: HashFunc) {
        self._hash = Some(hash_func)
    }

    pub fn with_iter(&mut self, iter_func: IterFunc) {
        self._iter = Some(iter_func);
    }

    pub fn with_next(&mut self, next_func: NextFunc) {
        self._next = Some(next_func);
    }
}

// dyn
#[allow(dead_code)]
impl RType {
    pub(crate) fn with_new_dyn(&mut self, new_func: RValue) {
        self._new_dyn = Some(new_func);
    }

    pub(crate) fn with_destory_dyn(&mut self, destory: RValue) {
        self._destory_dyn = Some(destory);
    }

    pub(crate) fn with_get_attr_dyn(&mut self, get_attr_func: RValue) {
        self._get_attr_dyn = Some(get_attr_func);
    }

    pub(crate) fn with_set_attr_dyn(&mut self, set_attr_func: RValue) {
        self._set_attr_dyn = Some(set_attr_func);
    }

    pub(crate) fn with_get_item_dyn(&mut self, get_item_func: RValue) {
        self._get_item_dyn = Some(get_item_func);
    }

    pub(crate) fn with_set_item_dyn(&mut self, set_item_func: RValue) {
        self._set_item_dyn = Some(set_item_func);
    }

    pub(crate) fn with_call_dyn(&mut self, call_func: RValue) {
        self._call_dyn = Some(call_func);
    }

    pub(crate) fn with_arith_dyn(&mut self, op: ArithOp, op_func: RValue) {
        self._arith_dyn[op as usize] = Some(op_func);
    }

    pub(crate) fn with_unary_dyn(&mut self, op: UnaryOp, op_func: RValue) {
        self._unary_dyn[op as usize] = Some(op_func);
    }

    pub(crate) fn with_cmp_dyn(&mut self, cmp: RValue) {
        self._cmp_dyn = Some(cmp)
    }

    pub(crate) fn with_eq_dyn(&mut self, eq: RValue) {
        self._eq_dyn = Some(eq)
    }

    pub(crate) fn with_str_dyn(&mut self, str_func: RValue) {
        self._str_dyn = Some(str_func)
    }

    pub(crate) fn with_hash_dyn(&mut self, hash_func: RValue) {
        self._hash_dyn = Some(hash_func)
    }

    pub(crate) fn with_iter_dyn(&mut self, iter_func: RValue) {
        self._iter_dyn = Some(iter_func);
    }

    pub(crate) fn with_next_dyn(&mut self, next_func: RValue) {
        self._next_dyn = Some(next_func);
    }
}

impl Ref<RType> {
    #[inline]
    pub fn set_attr(&mut self, name: &Ref<RString>, attr: RValue) -> Result<(), Error> {
        self._attrs.insert(name.clone(), attr).map(|_| ())
    }

    #[inline]
    pub fn set_attr_str(&mut self, name: &str, attr: RValue) -> Result<(), Error> {
        let name = RString::new(name)?;
        self._attrs.insert(name, attr).map(|_| ())
    }

    #[inline]
    pub fn get_attr(&self, name: &Ref<RString>) -> Option<&RValue> {
        self._attrs.get(name)
    }

    #[inline]
    pub fn add_method_str_light(&mut self, name: &str, method: RRustFunction) -> Result<(), Error> {
        let name = RString::new(name)?;
        let method = RFunction::from_rust_func(method)?;
        self._attrs.insert(name, method.cast_value()).map(|_| ())
    }
}

impl Debug for RType {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_fmt(format_args!("<Type \"{}\">", self.name().as_str()))
    }
}

pub(crate) fn _init_type_type(mut tp: Ref<RType>) -> Result<(), Error> {
    tp.with_visit(type__visit);

    tp.with_destory(type__destory);

    tp.with_eq(default_value_eq);
    tp.with_str(default_value_str);
    tp.with_hash(default_value_hash);

    tp.with_get_attr(type__get_attr);
    tp.with_set_attr(type__set_attr);

    tp.with_call(type__call);

    tp.add_method_str_light("name", type__name)?;

    Ok(())
}

use crate::runtime::Visitor;
use crate::util::expect_arg1;

#[allow(non_snake_case)]
fn type__visit(visitor: &mut dyn Visitor, value_ptr: NonNull<GcHeader>) {
    unsafe {
        let tp = value_ptr.cast::<RType>().as_ref();

        visitor.visit_value(tp._name.cast_value_ref());
        for (k, _) in tp._attrs.iter() {
            visitor.visit_value(k.cast_value_ref());
        }

        if let Some(v) = &tp._new_dyn {
            visitor.visit_value(v.cast_value_ref());
        }
        if let Some(v) = &tp._destory_dyn {
            visitor.visit_value(v.cast_value_ref());
        }
        if let Some(v) = &tp._get_attr_dyn {
            visitor.visit_value(v.cast_value_ref());
        }
        if let Some(v) = &tp._set_attr_dyn {
            visitor.visit_value(v.cast_value_ref());
        }
        if let Some(v) = &tp._get_item_dyn {
            visitor.visit_value(v.cast_value_ref());
        }
        if let Some(v) = &tp._set_item_dyn {
            visitor.visit_value(v.cast_value_ref());
        }
        if let Some(v) = &tp._call_dyn {
            visitor.visit_value(v.cast_value_ref());
        }
        if let Some(v) = &tp._eq_dyn {
            visitor.visit_value(v.cast_value_ref());
        }
        if let Some(v) = &tp._cmp_dyn {
            visitor.visit_value(v.cast_value_ref());
        }
        if let Some(v) = &tp._str_dyn {
            visitor.visit_value(v.cast_value_ref());
        }
        if let Some(v) = &tp._hash_dyn {
            visitor.visit_value(v.cast_value_ref());
        }
        if let Some(v) = &tp._iter_dyn {
            visitor.visit_value(v.cast_value_ref());
        }
        if let Some(v) = &tp._next_dyn {
            visitor.visit_value(v.cast_value_ref());
        }

        for op in &tp._arith_dyn {
            if let Some(v) = op {
                visitor.visit_value(v.cast_value_ref());
            }
        }
        for op in &tp._unary_dyn {
            if let Some(v) = op {
                visitor.visit_value(v.cast_value_ref());
            }
        }
    }
}

#[allow(non_snake_case)]
fn type__destory(value: &RValue) -> Result<(), Error> {
    unsafe {
        let mut tp = value.expect_cast::<RType>(type_type())?;
        tp._drop();
    };
    Ok(())
}

#[allow(non_snake_case)]
fn type__get_attr(value: &RValue, name: &Ref<RString>) -> Result<RValue, Error> {
    let tp = unsafe { value.expect_cast::<RType>(type_type())? };
    if let Some(v) = tp._attrs.get(name) {
        Ok(v.clone())
    } else {
        Err(runtime_error_fmt!(
            "{:?} has no attribute \"{}\"",
            &value,
            name.as_str()
        ))
    }
}
#[allow(non_snake_case)]
fn type__set_attr(value: &RValue, name: &Ref<RString>, attr_value: &RValue) -> Result<(), Error> {
    let mut tp = unsafe { value.expect_cast::<RType>(type_type())? };
    tp._attrs
        .insert(name.clone(), attr_value.clone())
        .map(|_| ())
}

#[allow(non_snake_case)]
fn type__call(callee: &RValue, this: &RValue, args: &[RValue]) -> Result<RValue, Error> {
    let callee_tp = unsafe { callee.expect_cast::<RType>(type_type())? };
    if Ref::ptr_eq(&callee_tp, type_type()) {
        _type_type__call(&callee_tp, this, args)
    } else {
        _other_type__call(&callee_tp, this, args)
    }
}

#[inline]
#[allow(non_snake_case)]
fn _type_type__call(
    _callee: &Ref<RType>,
    _this: &RValue,
    args: &[RValue],
) -> Result<RValue, Error> {
    let arg = expect_arg1(args)?;
    Ok(arg.get_type().cast_value())
}

#[inline]
#[allow(non_snake_case)]
fn _other_type__call(
    callee: &Ref<RType>,
    _this: &RValue,
    args: &[RValue],
) -> Result<RValue, Error> {
    value_new(callee, args)
}

#[allow(non_snake_case)]
fn type__name(this: &RValue, _args: &[RValue]) -> Result<RValue, Error> {
    let tp = unsafe { this.expect_cast::<RType>(type_type())? };
    Ok(tp.name().cast_value())
}
