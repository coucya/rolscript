#![allow(dead_code)]

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArithOp {
    Add = 0,
    Sub,
    Mul,
    Div,
    IDiv,
    Mod,
    Pow,
    And,
    Or,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
}

pub const ARITH_OP_COUNT: usize = ArithOp::Shr as usize + 1;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CmpOp {
    Cmp = 0,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

pub const CMP_OP_COUNT: usize = CmpOp::Ge as usize + 1;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnaryOp {
    Not = 0,
    BitNot,
}

pub const UNARY_OP_COUNT: usize = UnaryOp::BitNot as usize + 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum OverloadOp {
    New = 0,
    Destory,
    Str,
    Hash,
    Iter,
    Next,
    GetItem,
    SetItem,
    Call,
    Eq,
    Cmp,
    Add,
    Sub,
    Mul,
    Div,
    IDiv,
    Mod,
    Pow,
    And,
    Or,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    Not,
    BitNot,
}

pub const OVERLOAD_OP_COUNT: usize = OverloadOp::BitNot as usize + 1;

impl OverloadOp {
    pub fn from_u8(n: u8) -> Option<Self> {
        if n <= Self::BitNot as u8 {
            use core::mem::transmute;
            unsafe { Some(transmute(n)) }
        } else {
            None
        }
    }
    pub fn is_arith(&self) -> bool {
        *self as u8 >= Self::Add as u8 && *self as u8 <= Self::Shr as u8
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Opcode {
    Nop,
    LoadNull,
    LoadTrue,
    LoadFalse,
    LoadInt(i32),
    LoadConstStr(u32),
    LoadConstNum(u32),
    LoadThis,
    NewTuple(u32),
    NewArray(u32),
    NewMap(u32),
    NewClosure(u32),
    NewType,
    SetOverload(u8), // (overload_op)
    GetCapture(u32),
    SetCapture(u32),
    GetLocal(u32),
    SetLocal(u32),
    GetGlobal(u32),
    GetAttr(u32),
    GetAttrDup(u32),
    SetAttr(u32),
    GetItem,
    SetItem,
    Add,
    Sub,
    Mul,
    Div,
    IDiv,
    Mod,
    Pow,
    And,
    Or,
    Not,
    BitAnd,
    BitOr,
    BitXor,
    BitNot,
    Shl,
    Shr,
    Cmp,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Iter,
    IfFalse(i32),
    Jmp(i32),
    /// 该指令用于for循环。   
    /// 弹出栈顶值a，使 v = next(a)，    
    /// 当v为ROption::none时，使pc加上指令附带的值并跳转。   
    /// 当v为ROption::some时，把其内部的值取出并放入栈顶。   
    IterNext(i32),
    IfFalseLabel(u32),
    JmpLabel(u32),
    IterNextLabel(u32),
    Call(u32),
    CallThis(u32),
    CallMethod(u16, u16), // (nmae index, arg count)
    CallAttr(u16, u16),
    Apply(u32),
    Return,
    Pop,
    Dup,
    Rot,
    Rot3,
    Rot4,
}
pub mod opcode_funcs {
    use crate::op::*;

    use crate::error::*;
    use crate::runtime_error_fmt;

    use crate::array::*;
    use crate::function::*;
    use crate::map::*;
    use crate::number::*;
    use crate::tuple::*;
    use crate::type_::*;
    use crate::value::*;

    use crate::builtin::*;

    #[inline]
    pub(crate) fn new_tuple(args: &[RValue]) -> Result<RValue, Error> {
        Ok(RTuple::from_slice(args)?.cast_value())
    }

    #[inline]
    pub(crate) fn new_array(args: &[RValue]) -> Result<RValue, Error> {
        let mut arr = RArray::new()?;

        for v in args {
            arr.push(v.clone())?;
        }

        Ok(arr.cast_value())
    }

    #[inline]
    pub(crate) fn new_map(args: &[RValue]) -> Result<RValue, Error> {
        let mut map = RMap::new()?;
        let n = args.len() / 2;
        for i in 0..n {
            let k = args[i * 2].clone();
            let v = args[i * 2 + 1].clone();
            map.set(k, v)?;
        }

        Ok(map.cast_value())
    }

    #[inline]
    pub(crate) fn new_closure(
        parent: &Ref<RFunction>,
        idx: u32,
        captured: RValue,
    ) -> Result<RValue, Error> {
        let func = unsafe { parent.expect_cast::<RFunction>(function_type())? };
        let parent_code = if let Some(code) = func.get_code() {
            code
        } else {
            return Err(runtime_error_fmt!(
                "invalid function type when create new closure"
            ));
        };

        let code = if let Some(code) = parent_code.get_child(idx as usize) {
            code
        } else {
            return Err(runtime_error_fmt!(
                "invalid child function index when create new closure"
            ));
        };

        // TODO: 元组？
        if captured.is_type(array_type()) {
            let caps = unsafe { captured.cast_ref::<RArray>() };
            let func = RFunction::from_script_code(code, caps.clone())?;
            Ok(func.cast_value())
        } else {
            Err(runtime_error_fmt!("capture variable list must be Array"))
        }
    }

    #[inline]
    pub(crate) fn set_overload(
        oop: OverloadOp,
        tp: &mut Ref<RType>,
        func: &RValue,
    ) -> Result<(), Error> {
        let func = func.clone();
        match oop {
            OverloadOp::New => tp.with_new_dyn(func),
            OverloadOp::Destory => tp.with_destory_dyn(func),
            OverloadOp::Str => tp.with_str_dyn(func),
            OverloadOp::Hash => tp.with_hash_dyn(func),
            OverloadOp::Iter => tp.with_iter_dyn(func),
            OverloadOp::Next => tp.with_next_dyn(func),
            OverloadOp::GetItem => tp.with_get_item_dyn(func),
            OverloadOp::SetItem => tp.with_set_item_dyn(func),
            OverloadOp::Call => tp.with_call_dyn(func),
            OverloadOp::Eq => tp.with_eq_dyn(func),
            OverloadOp::Cmp => tp.with_cmp_dyn(func),
            OverloadOp::Add => tp.with_arith_dyn(ArithOp::Add, func),
            OverloadOp::Sub => tp.with_arith_dyn(ArithOp::Sub, func),
            OverloadOp::Mul => tp.with_arith_dyn(ArithOp::Mul, func),
            OverloadOp::Div => tp.with_arith_dyn(ArithOp::Div, func),
            OverloadOp::IDiv => tp.with_arith_dyn(ArithOp::IDiv, func),
            OverloadOp::Mod => tp.with_arith_dyn(ArithOp::Mod, func),
            OverloadOp::Pow => tp.with_arith_dyn(ArithOp::Pow, func),
            OverloadOp::And => tp.with_arith_dyn(ArithOp::And, func),
            OverloadOp::Or => tp.with_arith_dyn(ArithOp::Or, func),
            OverloadOp::BitAnd => tp.with_arith_dyn(ArithOp::BitAnd, func),
            OverloadOp::BitOr => tp.with_arith_dyn(ArithOp::BitOr, func),
            OverloadOp::BitXor => tp.with_arith_dyn(ArithOp::BitXor, func),
            OverloadOp::Shl => tp.with_arith_dyn(ArithOp::Shl, func),
            OverloadOp::Shr => tp.with_arith_dyn(ArithOp::Shr, func),
            OverloadOp::Not => tp.with_unary_dyn(UnaryOp::Not, func),
            OverloadOp::BitNot => tp.with_unary_dyn(UnaryOp::BitNot, func),
        }
        Ok(())
    }

    #[inline]
    pub(crate) fn set_capture(closure: RValue, idx: u32, value: RValue) -> Result<(), Error> {
        let mut func = unsafe { closure.expect_cast::<RFunction>(function_type())? };
        func.set_captured(idx as u64, value);
        Ok(())
    }

    #[inline]
    pub(crate) fn cmp(left: &RValue, right: &RValue) -> Result<RValue, Error> {
        let n = value_cmp(left, right)?;
        RInt::new(n).map(|v| v.cast_value())
    }
    #[inline]
    pub(crate) fn eq(left: &RValue, right: &RValue) -> Result<RValue, Error> {
        let b = value_eq(left, right)?;
        if b {
            Ok(true_().cast_value())
        } else {
            Ok(true_().cast_value())
        }
    }
    #[inline]
    pub(crate) fn ne(left: &RValue, right: &RValue) -> Result<RValue, Error> {
        let b = value_eq(left, right)?;
        if b {
            Ok(false_().cast_value())
        } else {
            Ok(true_().cast_value())
        }
    }
    #[inline]
    pub(crate) fn lt(left: &RValue, right: &RValue) -> Result<RValue, Error> {
        let c = value_cmp(left, right)?;
        if c < 0 {
            Ok(true_().cast_value())
        } else {
            Ok(false_().cast_value())
        }
    }
    #[inline]
    pub(crate) fn le(left: &RValue, right: &RValue) -> Result<RValue, Error> {
        let c = value_cmp(left, right)?;
        if c <= 0 {
            Ok(true_().cast_value())
        } else {
            Ok(false_().cast_value())
        }
    }
    #[inline]
    pub(crate) fn gt(left: &RValue, right: &RValue) -> Result<RValue, Error> {
        let c = value_cmp(left, right)?;
        if c > 0 {
            Ok(true_().cast_value())
        } else {
            Ok(false_().cast_value())
        }
    }
    #[inline]
    pub(crate) fn ge(left: &RValue, right: &RValue) -> Result<RValue, Error> {
        let c = value_cmp(left, right)?;
        if c >= 0 {
            Ok(true_().cast_value())
        } else {
            Ok(false_().cast_value())
        }
    }
}
