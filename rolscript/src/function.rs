use core::mem::size_of;
use core::ptr::addr_of_mut;
use core::ptr::NonNull;

use crate::collections::Array;

use crate::runtime::*;

use crate::array::RArray;
use crate::number::*;
use crate::script_code::*;
use crate::string::*;
use crate::type_::*;
use crate::value::*;

use crate::builtin::*;

use crate::op::*;

use crate::error::*;
use crate::runtime_error_fmt;

pub type RRustFunction = fn(&RValue, &[RValue]) -> Result<RValue, Error>;

#[derive(Clone, Copy, PartialEq, Eq)]
enum FuncType {
    Script,
    Rust,
    Native,
}

struct _ScriptFunc {
    code: Ref<RScriptCode>,
    captured: Ref<RArray>,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct _NativeCallableBase {
    _call: fn(*mut _NativeCallableBase, &RValue, &[RValue]) -> Result<RValue, Error>,
    _drop: fn(*mut _NativeCallableBase),
}

impl _NativeCallableBase {
    fn new(
        call: fn(*mut _NativeCallableBase, &RValue, &[RValue]) -> Result<RValue, Error>,
        drop: fn(*mut _NativeCallableBase),
    ) -> Self {
        Self {
            _call: call,
            _drop: drop,
        }
    }

    fn call(&mut self, this: &RValue, args: &[RValue]) -> Result<RValue, Error> {
        (self._call)(self as _, this, args)
    }

    fn drop(&mut self) {
        (self._drop)(self as _);
    }
}

#[repr(C)]
struct _NativeCallable<F>
where
    F: FnMut(&RValue, &[RValue]) -> Result<RValue, Error>,
{
    pub(self) base: _NativeCallableBase,
    pub(self) callable: F,
}

impl<F> _NativeCallable<F>
where
    F: FnMut(&RValue, &[RValue]) -> Result<RValue, Error>,
{
    pub(self) fn new(callable: F) -> Self {
        Self {
            base: _NativeCallableBase::new(Self::_call, Self::_drop),
            callable,
        }
    }

    fn _call(
        self_: *mut _NativeCallableBase,
        this: &RValue,
        args: &[RValue],
    ) -> Result<RValue, Error> {
        unsafe {
            let self_ = self_.cast::<_NativeCallable<F>>();
            ((&mut *self_).callable)(this, args)
        }
    }

    fn _drop(self_: *mut _NativeCallableBase) {
        unsafe {
            let self_ = self_.cast::<_NativeCallable<F>>();
            self_.drop_in_place();
        }
    }
}

use core::mem::ManuallyDrop;

#[repr(C)]
union Function {
    script: ManuallyDrop<_ScriptFunc>,
    rust: RRustFunction,
    native: _NativeCallableBase,
}

#[repr(C)]
pub struct RFunction {
    _header: GcHeader,
    _type: FuncType,
    _func: Function,
}

impl RFunction {
    unsafe fn init_script(mut ptr: NonNull<Self>, func: _ScriptFunc) {
        addr_of_mut!(ptr.as_mut()._type).write(FuncType::Script);
        addr_of_mut!(ptr.as_mut()._func).write(Function {
            script: ManuallyDrop::new(func),
        });
    }

    unsafe fn init_rust(mut ptr: NonNull<Self>, func: RRustFunction) {
        addr_of_mut!(ptr.as_mut()._type).write(FuncType::Rust);
        addr_of_mut!(ptr.as_mut()._func).write(Function { rust: func });
    }

    unsafe fn init_native<F>(mut ptr: NonNull<Self>, func: F)
    where
        F: FnMut(&RValue, &[RValue]) -> Result<RValue, Error>,
    {
        addr_of_mut!(ptr.as_mut()._type).write(FuncType::Native);

        let native_ptr = addr_of_mut!(ptr.as_mut()._func.native).cast::<_NativeCallable<F>>();
        let native_func = _NativeCallable::new(func);
        native_ptr.write(native_func);
    }

    unsafe fn _drop(&mut self) {
        match self._type {
            FuncType::Native => self.as_native_mut().drop(),
            FuncType::Script => {
                addr_of_mut!(self.as_script_mut().code).drop_in_place();
                addr_of_mut!(self.as_script_mut().captured).drop_in_place();
            }
            FuncType::Rust => {}
        }
    }

    pub fn from_script_code(
        code: Ref<RScriptCode>,
        captured: Ref<RArray>,
    ) -> Result<Ref<RFunction>, Error> {
        unsafe {
            let tp = function_type().clone();
            let size = size_of::<Self>();
            let v = new_gc_obj(size, tp)?.cast::<Self>();
            Self::init_script(v.as_nonnull_ptr(), _ScriptFunc { code, captured });
            Ok(v)
        }
    }

    pub fn from_rust_func(func: RRustFunction) -> Result<Ref<RFunction>, Error> {
        unsafe {
            let tp = function_type().clone();
            let size = size_of::<Self>();
            let v = new_gc_obj(size, tp)?.cast::<Self>();
            Self::init_rust(v.as_nonnull_ptr(), func);
            Ok(v)
        }
    }

    pub fn from_callable<F>(callable: F) -> Result<Ref<RFunction>, Error>
    where
        F: FnMut(&RValue, &[RValue]) -> Result<RValue, Error>,
    {
        unsafe {
            let tp = function_type().clone();
            // 消耗的内存多了就多了吧，懒得仔细算了。
            let size = size_of::<Self>() + size_of::<F>();
            let v = new_gc_obj(size, tp)?.cast::<Self>();
            Self::init_native(v.as_nonnull_ptr(), callable);
            Ok(v)
        }
    }

    pub(self) unsafe fn as_rust(&self) -> RRustFunction {
        self._func.rust
    }

    pub(self) unsafe fn as_script(&self) -> &_ScriptFunc {
        &*self._func.script
    }

    pub(self) unsafe fn as_script_mut(&mut self) -> &mut _ScriptFunc {
        &mut *self._func.script
    }

    #[allow(dead_code)]
    pub(self) unsafe fn as_native(&self) -> &_NativeCallableBase {
        &self._func.native
    }

    pub(self) unsafe fn as_native_mut(&mut self) -> &mut _NativeCallableBase {
        &mut self._func.native
    }

    pub fn get_code(&self) -> Option<Ref<RScriptCode>> {
        if self._type == FuncType::Script {
            unsafe {
                let _ScriptFunc { code, captured: _ } = self.as_script();
                Some(code.clone())
            }
        } else {
            None
        }
    }

    pub fn captured_count(&self) -> usize {
        if self._type == FuncType::Script {
            unsafe {
                let _ScriptFunc { code: _, captured } = self.as_script();
                captured.len()
            }
        } else {
            0
        }
    }

    pub fn set_captured(&mut self, n: u64, value: RValue) {
        #[allow(unused_must_use)]
        if self._type == FuncType::Script {
            unsafe {
                let _ScriptFunc { code: _, captured } = self.as_script_mut();
                captured.set(n as Int, value);
            }
        }
    }
}

pub(crate) fn _init_type_function(mut tp: Ref<RType>) -> Result<(), Error> {
    tp.with_visit(function__visit);

    tp.with_destory(function__destory);

    tp.with_call(function__call);

    tp.with_eq(default_value_eq);
    tp.with_hash(default_value_hash);
    tp.with_str(default_value_str);

    Ok(())
}

use crate::runtime::Visitor;

#[allow(non_snake_case)]
fn function__visit(visitor: &mut dyn Visitor, value_ptr: NonNull<GcHeader>) {
    unsafe {
        let func = value_ptr.cast::<RFunction>().as_ref();
        match func._type {
            FuncType::Script => {
                let _ScriptFunc { code, captured } = func.as_script();

                visitor.visit_value(code.cast_value_ref());

                for cap in captured.as_slice() {
                    visitor.visit_value(cap);
                }
            }
            FuncType::Rust => (),
            FuncType::Native => {
                // TODO
            }
        }
    }
}

#[allow(non_snake_case)]
fn function__destory(value: &RValue) -> Result<(), Error> {
    unsafe {
        let mut func = value.expect_cast::<RFunction>(function_type())?;
        func._drop();
        Ok(())
    }
}

#[allow(non_snake_case)]
fn function__call(callee: &RValue, this_value: &RValue, args: &[RValue]) -> Result<RValue, Error> {
    let mut func = unsafe { callee.expect_cast::<RFunction>(function_type())? };
    unsafe {
        match func._type {
            FuncType::Rust => func.as_rust()(this_value, args),
            FuncType::Script => {
                let _ScriptFunc { code, captured } = func.as_script();
                eval_script_closure(&func, code, captured.as_slice(), this_value, args)
            }
            FuncType::Native => func.as_native_mut().call(this_value, args),
        }
    }
}

fn eval_script_closure(
    callee: &Ref<RFunction>,
    callee_code: &Ref<RScriptCode>,
    caps: &[RValue],
    this_value: &RValue,
    args: &[RValue],
) -> Result<RValue, Error> {
    use opcode_funcs as opfunc;
    use Opcode::*;

    #[inline]
    fn push(stack: &mut Array<RValue>, v: RValue) -> Result<(), Error> {
        stack.push(v).map_err(|_| Error::OutOfMemory)
    }
    #[inline]
    fn pop(stack: &mut Array<RValue>) -> Result<RValue, Error> {
        stack
            .pop()
            .ok_or_else(|| runtime_error_fmt!("pop form empty stack"))
    }
    #[inline]
    fn pop_n(stack: &mut Array<RValue>, n: usize) {
        for _ in 0..n {
            stack.pop();
        }
    }
    #[inline]
    fn top(stack: &mut Array<RValue>) -> Result<RValue, Error> {
        if stack.len() != 0 {
            Ok(stack.get(stack.len() - 1).cloned().unwrap())
        } else {
            Err(runtime_error_fmt!("top form empty stack"))
        }
    }

    #[inline]
    fn get_local(stack: &mut Array<RValue>, idx: usize) -> Result<RValue, Error> {
        if let Some(v) = stack.get(idx) {
            Ok(v.clone())
        } else {
            Err(runtime_error_fmt!("invalid local var idx"))
        }
    }

    #[inline]
    #[allow(unused_must_use)]
    fn set_local(stack: &mut Array<RValue>, idx: usize, val: RValue) {
        if idx < stack.len() {
            stack.set(idx, val);
        }
    }

    #[inline]
    fn lasts<'a>(stack: &'a mut Array<RValue>, n: usize) -> Result<&'a [RValue], Error> {
        if n > stack.len() {
            return Err(runtime_error_fmt!("stack error"));
        }
        let start = stack.len() - n;
        Ok(&stack.as_slice()[start..])
    }

    #[inline]
    fn get_const_str(code: &Ref<RScriptCode>, index: usize) -> Result<Ref<RString>, Error> {
        code.get_const_string(index)
            .ok_or_else(|| runtime_error_fmt!("invalid const string index"))
    }

    let local_count = callee_code.local_count();
    let paramet_count = callee_code.paramet_count() as usize;
    let ops = callee_code.opcode();
    let ret;

    let local_stack_count = args.len() + local_count;

    let mut stack_ = Array::new(allocator());
    stack_
        .reserve(local_stack_count)
        .map_err(|_| Error::OutOfMemory)?;
    let stack = &mut stack_;

    // TODO: 收集剩余参数到数组。
    for v in args {
        push(stack, v.clone())?;
    }

    let null_value = null().cast_value();
    for _ in paramet_count..local_count {
        push(stack, null_value.clone())?;
    }

    let mut ip: usize = 0;
    let mut offset: i32 = 0;

    // println!("===== NEW Func =====");

    #[allow(unused_variables)]
    loop {
        let op = ops[ip];

        // println!("{:?} ", stack.as_slice());
        // println!("    | V | {:?}", op);

        match op {
            Nop => (),
            LoadNull => {
                let v = null().cast_value();
                push(stack, v)?
            }
            LoadTrue => {
                let v = true_().cast_value();
                push(stack, v)?
            }
            LoadFalse => {
                let v = false_().cast_value();
                push(stack, v)?
            }
            LoadInt(n) => {
                let v = RInt::new(n as Int)?.cast_value();
                push(stack, v)?
            }
            LoadConstStr(idx) => {
                let v = get_const_str(callee_code, idx as usize)?;
                push(stack, v.cast_value())?
            }
            LoadConstNum(idx) => {
                let v = if let Some(n) = callee_code.get_const_number(idx as usize) {
                    n
                } else {
                    return Err(runtime_error_fmt!("invalid const number index"));
                };
                push(stack, v)?
            }
            LoadThis => push(stack, this_value.clone())?,
            NewTuple(count) => {
                let vs = lasts(stack, count as usize)?;
                let v = opfunc::new_tuple(vs)?;
                pop_n(stack, count as usize);
                push(stack, v)?;
            }
            NewArray(count) => {
                let vs = lasts(stack, count as usize)?;
                let v = opfunc::new_array(vs)?;
                pop_n(stack, count as usize);
                push(stack, v)?;
            }
            NewMap(count) => {
                let vs = lasts(stack, count as usize * 2)?;
                let v = opfunc::new_map(vs)?;
                pop_n(stack, count as usize * 2);
                push(stack, v)?;
            }
            NewClosure(idx) => {
                let captured = pop(stack)?;
                let v = opfunc::new_closure(callee, idx, captured)?;
                push(stack, v)?;
            }
            NewType => {
                use crate::dyn_::*;
                // let init_func = pop(stack)?;
                let name = pop(stack)?;
                let name_str = if name.is_type(string_type()) {
                    unsafe { name.cast_ref::<RString>() }
                } else {
                    return Err(runtime_error_fmt!("type name must be string"));
                };
                let new_type = type_new_dyn(&name_str)?;
                // value_call_with_this( &init_func, &new_type.cast_value(), &[])?;
                push(stack, new_type.cast_value())?;
            }
            SetOverload(oop) => {
                let func = pop(stack)?;
                let target = pop(stack)?;
                let mut tp = unsafe { target.expect_cast::<RType>(type_type())? };
                if let Some(oop) = OverloadOp::from_u8(oop) {
                    opfunc::set_overload(oop, &mut tp, &func)?;
                } else {
                    return Err(runtime_error_fmt!("invalid overload op"));
                }
            }
            GetCapture(idx) => {
                let v = caps
                    .get(idx as usize)
                    .cloned()
                    .unwrap_or(null().cast_value());
                push(stack, v)?;
            }
            SetCapture(idx) => {
                let value = pop(stack)?;
                let target_closure = pop(stack)?;
                opfunc::set_capture(target_closure, idx, value)?;
            }
            GetGlobal(idx) => {
                let name = get_const_str(callee_code, idx as usize)?;
                let value = if let Some(v) = get_global(&name) {
                    v
                } else {
                    return Err(runtime_error_fmt!(
                        "field \"{}\" does not exist in Global",
                        name.as_str(),
                    ));
                };
                push(stack, value)?;
            }
            GetLocal(idx) => {
                let v = get_local(stack, idx as usize)?;
                push(stack, v)?;
            }
            SetLocal(idx) => {
                let v = pop(stack)?;
                set_local(stack, idx as usize, v);
            }
            GetAttr(idx) => {
                let target = pop(stack)?;
                let name = get_const_str(callee_code, idx as usize)?;
                let v = value_get_attr(&target, &name)?;
                push(stack, v)?;
            }
            GetAttrDup(idx) => {
                let target = top(stack)?;
                let name = get_const_str(callee_code, idx as usize)?;
                let v = value_get_attr(&target, &name)?;
                push(stack, v)?;
            }
            SetAttr(idx) => {
                let value = pop(stack)?;
                let target = pop(stack)?;
                let name = get_const_str(callee_code, idx as usize)?;
                let v = value_set_attr(&target, &name, &value)?;
            }
            GetItem => {
                let idx = pop(stack)?;
                let target = pop(stack)?;
                let v = value_get_item(&target, &idx)?;
                push(stack, v)?;
            }
            SetItem => {
                let value = pop(stack)?;
                let idx = pop(stack)?;
                let target = pop(stack)?;
                value_set_item(&target, &idx, &value)?;
            }
            Add => {
                let right = pop(stack)?;
                let left = pop(stack)?;
                let v = value_binary_op(ArithOp::Add, &left, &right)?;
                push(stack, v)?;
            }
            Sub => {
                let right = pop(stack)?;
                let left = pop(stack)?;
                let v = value_binary_op(ArithOp::Sub, &left, &right)?;
                push(stack, v)?;
            }
            Mul => {
                let right = pop(stack)?;
                let left = pop(stack)?;
                let v = value_binary_op(ArithOp::Mul, &left, &right)?;
                push(stack, v)?;
            }
            Div => {
                let right = pop(stack)?;
                let left = pop(stack)?;
                let v = value_binary_op(ArithOp::Div, &left, &right)?;
                push(stack, v)?;
            }
            IDiv => {
                let right = pop(stack)?;
                let left = pop(stack)?;
                let v = value_binary_op(ArithOp::IDiv, &left, &right)?;
                push(stack, v)?;
            }
            Mod => {
                let right = pop(stack)?;
                let left = pop(stack)?;
                let v = value_binary_op(ArithOp::Mod, &left, &right)?;
                push(stack, v)?;
            }
            Pow => {
                let right = pop(stack)?;
                let left = pop(stack)?;
                let v = value_binary_op(ArithOp::Pow, &left, &right)?;
                push(stack, v)?;
            }
            And => {
                let right = pop(stack)?;
                let left = pop(stack)?;
                let v = value_binary_op(ArithOp::And, &left, &right)?;
                push(stack, v)?;
            }
            Or => {
                let right = pop(stack)?;
                let left = pop(stack)?;
                let v = value_binary_op(ArithOp::Or, &left, &right)?;
                push(stack, v)?;
            }
            Not => {
                let right = pop(stack)?;
                let v = value_unary_op(UnaryOp::Not, &right)?;
                push(stack, v)?;
            }
            BitAnd => {
                let right = pop(stack)?;
                let left = pop(stack)?;
                let v = value_binary_op(ArithOp::BitAnd, &left, &right)?;
                push(stack, v)?;
            }
            BitOr => {
                let right = pop(stack)?;
                let left = pop(stack)?;
                let v = value_binary_op(ArithOp::BitOr, &left, &right)?;
                push(stack, v)?;
            }
            BitXor => {
                let right = pop(stack)?;
                let left = pop(stack)?;
                let v = value_binary_op(ArithOp::BitXor, &left, &right)?;
                push(stack, v)?;
            }
            BitNot => {
                let right = pop(stack)?;
                let v = value_unary_op(UnaryOp::BitNot, &right)?;
                push(stack, v)?;
            }
            Shl => {
                let right = pop(stack)?;
                let left = pop(stack)?;
                let v = value_binary_op(ArithOp::Shl, &left, &right)?;
                push(stack, v)?;
            }
            Shr => {
                let right = pop(stack)?;
                let left = pop(stack)?;
                let v = value_binary_op(ArithOp::Shr, &left, &right)?;
                push(stack, v)?;
            }
            Cmp => {
                let right = pop(stack)?;
                let left = pop(stack)?;
                let v = opfunc::cmp(&left, &right)?;
                push(stack, v)?;
            }
            Eq => {
                let right = pop(stack)?;
                let left = pop(stack)?;
                let v = opfunc::eq(&left, &right)?;
                push(stack, v)?;
            }
            Ne => {
                let right = pop(stack)?;
                let left = pop(stack)?;
                let v = opfunc::ne(&left, &right)?;
                push(stack, v)?;
            }
            Lt => {
                let right = pop(stack)?;
                let left = pop(stack)?;
                let v = opfunc::lt(&left, &right)?;
                push(stack, v)?;
            }
            Le => {
                let right = pop(stack)?;
                let left = pop(stack)?;
                let v = opfunc::le(&left, &right)?;
                push(stack, v)?;
            }
            Gt => {
                let right = pop(stack)?;
                let left = pop(stack)?;
                let v = opfunc::gt(&left, &right)?;
                push(stack, v)?;
            }
            Ge => {
                let right = pop(stack)?;
                let left = pop(stack)?;
                let v = opfunc::ge(&left, &right)?;
                push(stack, v)?;
            }
            Iter => {
                let v = pop(stack)?;
                let iter = value_iter(&v)?;
                push(stack, iter)?;
            }
            IfFalse(offset_) => {
                let v = pop(stack)?;

                let is_null = v.is_type(&null_type());
                let is_false =
                    v.is_type(&bool_type()) && unsafe { v.cast_ref::<RBool>().as_bool() };
                offset = if is_null || is_false { 0 } else { offset_ - 1 };
            }
            Jmp(offset_) => offset = offset_ - 1,
            IterNext(offset_) => {
                let a = pop(stack)?;
                let v = value_next(&a)?;

                if let Some(inner_v) = v.value() {
                    push(stack, inner_v.clone())?;
                } else {
                    offset = offset_ - 1;
                }
            }
            IfFalseLabel(_) => Err(runtime_error_fmt!(
                "\"IfFalseLabel\" instruction is reserved",
            ))?,
            JmpLabel(_) => Err(runtime_error_fmt!("\"JmpLabel\" instruction is reserved"))?,
            IterNextLabel(_) => Err(runtime_error_fmt!(
                "\"IterNextLabel\" instruction is reserved"
            ))?,
            Call(count) => {
                let l = lasts(stack, count as usize + 1)?;
                let callee = l[0].clone();
                let ret = value_call(&callee, &l[1..])?;
                pop_n(stack, count as usize + 1);
                push(stack, ret)?;
            }
            CallThis(count) => {
                let l = lasts(stack, count as usize + 2)?;
                let this_value = l[0].clone();
                let callee = l[1].clone();
                let ret = value_call_with_this(&callee, &this_value, &l[2..])?;
                pop_n(stack, count as usize + 2);
                push(stack, ret)?;
            }
            CallMethod(idx, count) => {
                let l = lasts(stack, count as usize + 1)?;
                let this_value = l[0].clone();
                let name = get_const_str(callee_code, idx as usize)?;
                let ret = value_call_method(&this_value, &name, &l[1..])?;
                pop_n(stack, count as usize + 1);
                push(stack, ret)?;
            }
            CallAttr(idx, count) => {
                let l = lasts(stack, count as usize + 1)?;
                let this_value = l[0].clone();
                let name = get_const_str(callee_code, idx as usize)?;
                let ret = value_call_attr(&this_value, &name, &l[1..])?;
                pop_n(stack, count as usize + 1);
                push(stack, ret)?;
            }
            Apply(_) => Err(runtime_error_fmt!("\"Apply\" instruction is reserved"))?,
            Return => {
                ret = pop(stack)?;
                break;
            }
            Pop => {
                pop(stack)?;
            }
            Dup => {
                let v = top(stack)?;
                push(stack, v)?;
            }
            Rot => {
                let b = pop(stack)?;
                let a = pop(stack)?;
                push(stack, a)?;
                push(stack, b)?;
            }
            Rot3 => {
                let c = pop(stack)?;
                let b = pop(stack)?;
                let a = pop(stack)?;
                push(stack, c)?;
                push(stack, a)?;
                push(stack, b)?;
            }
            Rot4 => {
                let d = pop(stack)?;
                let c = pop(stack)?;
                let b = pop(stack)?;
                let a = pop(stack)?;
                push(stack, d)?;
                push(stack, a)?;
                push(stack, b)?;
                push(stack, c)?;
            }
        }

        let new_ip = ip as isize + offset as isize + 1;
        if new_ip < 0 || new_ip >= ops.len() as isize {
            Err(runtime_error_fmt!("exit without a instruction"))?
        }
        offset = 0;
        ip = new_ip as usize;
    }

    // println!("===== EXIT Func =====");

    Ok(ret)
}
