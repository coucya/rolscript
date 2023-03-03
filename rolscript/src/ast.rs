use core::fmt::Debug;
use core::fmt::{Formatter, Result as FmtResult};
use core::mem::size_of;
use core::ptr::addr_of_mut;
use core::ptr::NonNull;

use crate::collections::Array;

use crate::runtime::*;

use crate::op::*;

use crate::error::*;
use crate::runtime_error_fmt;

use crate::number::*;
use crate::script_code::RScriptCode;
use crate::script_code::ScriptCodeBuilder;
use crate::string::RString;
use crate::type_::*;
use crate::value::*;

use crate::builtin::*;

pub enum Ast {
    Int(Int),
    Float(Float),
    String(Ref<RString>),
    Tuple(Array<Ref<RAst>>),
    Array(Array<Ref<RAst>>),
    Map(Array<(Ref<RAst>, Ref<RAst>)>),
    Program {
        stats: Array<Ref<RAst>>,
        expr: Option<Ref<RAst>>,
    },
    ProgramPublic {
        name: Ref<RString>,
        expr: Ref<RAst>,
    },
    Block {
        stats: Array<Ref<RAst>>,
        expr: Option<Ref<RAst>>,
    },
    ArithExpr {
        op: ArithOp,
        left: Ref<RAst>,
        right: Ref<RAst>,
    },
    CmpExpr {
        op: CmpOp,
        left: Ref<RAst>,
        right: Ref<RAst>,
    },
    UnaryExpr {
        op: UnaryOp,
        expr: Ref<RAst>,
    },
    Lambda {
        paramets: Array<Ref<RString>>,
        body: Ref<RAst>,
    },
    FunctionDef {
        name: Ref<RString>,
        paramets: Array<Ref<RString>>,
        body: Ref<RAst>,
    },
    TypeDef {
        name: Ref<RString>,
        stats: Array<Ref<RAst>>,
    },
    OverloadDef {
        op: OverloadOp,
        paramets: Array<Ref<RString>>,
        body: Ref<RAst>,
    },
    TypePublic {
        name: Ref<RString>,
        expr: Ref<RAst>,
    },
    If {
        is_expr: bool,
        cond: Ref<RAst>,
        truebody: Ref<RAst>,
        falsebody: Option<Ref<RAst>>,
    },
    While {
        is_expr: bool,
        cond: Ref<RAst>,
        body: Ref<RAst>,
    },
    For {
        is_expr: bool,
        name: Ref<RString>,
        expr: Ref<RAst>,
        body: Ref<RAst>,
    },
    Ident {
        name: Ref<RString>,
    },
    Assign {
        target: Ref<RAst>,
        expr: Ref<RAst>,
    },
    Attr {
        expr: Ref<RAst>,
        name: Ref<RString>,
    },
    Index {
        expr: Ref<RAst>,
        index: Ref<RAst>,
    },
    Call {
        func: Ref<RAst>,
        args: Array<Ref<RAst>>,
    },
    MethodCall {
        target: Ref<RAst>,
        name: Ref<RString>,
        args: Array<Ref<RAst>>,
    },
    AttrCall {
        target: Ref<RAst>,
        name: Ref<RString>,
        args: Array<Ref<RAst>>,
    },
    Return {
        expr: Option<Ref<RAst>>,
    },
    Stat {
        expr: Option<Ref<RAst>>,
    },
}

impl Debug for Ref<RAst> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        fn write_slice<T: Debug>(
            f: &mut Formatter<'_>,
            beg: char,
            end: char,
            slice: &[T],
        ) -> FmtResult {
            write!(f, "{}", beg)?;
            if slice.len() > 0 {
                {
                    let v = &slice[0];
                    write!(f, "{:?}", v)?;
                }
                for v in &slice[1..] {
                    write!(f, ", {:?}", v)?;
                }
            }
            write!(f, "{}", end)
        }

        use Ast::*;
        match self.as_ast() {
            Int(n) => write!(f, "Int({})", n),
            Float(n) => write!(f, "Float({})", n),
            String(s) => write!(f, "String(\"{}\")", s.as_str()),
            Tuple(arr) => write!(f, "Tuple{:?}", arr.as_slice()),
            Array(arr) => write!(f, "Array{:?}", arr.as_slice()),
            Map(map) => {
                write!(f, "Map{{")?;
                if map.len() > 0 {
                    {
                        let (k, v) = &map.as_slice()[0];
                        write!(f, "{:?}: {:?}", k, v)?;
                    }
                    for (k, v) in &map.as_slice()[1..] {
                        write!(f, ", {:?}: {:?}", k, v)?;
                    }
                }
                write!(f, "}}")
            }
            Program { stats, expr } => {
                if let Some(expr) = expr {
                    write!(f, "Program{{")?;
                    write_slice(f, '{', '}', stats.as_slice())?;
                    write!(f, ", {:?}}}", expr)
                } else {
                    write!(f, "Program")?;
                    write_slice(f, '{', '}', stats.as_slice())
                }
            }
            ProgramPublic { name, expr } => {
                write!(f, "ProgramPublic {:?}{{{:?}}}", name, expr)
            }
            Block { stats, expr } => {
                if let Some(expr) = expr {
                    write!(f, "Block{{")?;
                    write_slice(f, '{', '}', stats.as_slice())?;
                    write!(f, ", {:?}}}", expr)
                } else {
                    write!(f, "Block")?;
                    write_slice(f, '{', '}', stats.as_slice())
                }
            }
            ArithExpr { op, left, right } => {
                write!(f, "ArithExpr({:?}){{{:?}, {:?}}}", op, left, right)
            }
            CmpExpr { op, left, right } => {
                write!(f, "CmpExpr({:?}){{{:?}, {:?}}}", op, left, right)
            }
            UnaryExpr { op, expr } => write!(f, "UnaryExpr({:?}){{{:?}}}", op, expr,),
            Lambda { paramets, body } => {
                write!(f, "Lambda")?;
                write_slice(f, '(', ')', paramets.as_slice())?;
                write!(f, "{{{:?}}}", body)
            }
            FunctionDef {
                name,
                paramets,
                body,
            } => {
                write!(f, "FunctionDef {:?}", name)?;
                write_slice(f, '(', ')', paramets.as_slice())?;
                write!(f, "{{{:?}}}", body)
            }
            TypeDef { name, stats } => write!(f, "TypeDef {:?} {:?}", name, stats.as_slice()),
            OverloadDef { op, paramets, body } => {
                write!(f, "OverloadDef {:?}", op)?;
                write_slice(f, '(', ')', paramets.as_slice())?;
                write!(f, "{{{:?}}}", body)
            }
            TypePublic { name, expr } => {
                write!(f, "TypePublic {:?}{{{:?}}}", name, expr)
            }
            If {
                is_expr,
                cond,
                truebody,
                falsebody,
            } => {
                if let Some(falsebody) = falsebody {
                    write!(
                        f,
                        "If({})({:?}) then {:?} else {:?}",
                        is_expr, cond, truebody, falsebody
                    )
                } else {
                    write!(f, "If({})({:?}) then {:?}", is_expr, cond, truebody)
                }
            }
            While {
                is_expr,
                cond,
                body,
            } => write!(f, "While({})({:?}) {:?}", is_expr, cond, body),

            For {
                is_expr,
                name,
                expr,
                body,
            } => write!(f, "For({})({:?} : {:?}) {:?}", is_expr, name, expr, body),
            Ident { name } => write!(f, "Ident(\"{:?}\")", name),
            Assign { target, expr } => {
                write!(f, "Assign{{{:?}={:?}}}", target, expr)
            }
            Attr { expr, name } => write!(f, "Attr{{{:?}.{:?}}}", expr, name),
            Index { expr, index } => write!(f, "Index{{{:?}[{:?}]}}", expr, index),
            Call { func, args } => {
                write!(f, "Call {:?}", func)?;
                write_slice(f, '(', ')', args.as_slice())
            }
            MethodCall { target, name, args } => {
                write!(f, "MethodCall {{{:?}}}.{:?}", target, name)?;
                write_slice(f, '(', ')', args.as_slice())
            }
            AttrCall { target, name, args } => {
                write!(f, "AttrCall {{{:?}}}::{:?}", target, name)?;
                write_slice(f, '(', ')', args.as_slice())
            }
            Return { expr } => {
                if let Some(expr) = expr {
                    write!(f, "Return {:?}", expr)
                } else {
                    write!(f, "Return")
                }
            }
            Stat { expr } => {
                if let Some(expr) = expr {
                    write!(f, "Stat {:?}", expr)
                } else {
                    write!(f, "Stat(;)")
                }
            }
        }
    }
}

#[repr(C)]
pub struct RAst {
    _header: GcHeader,
    _ast: Ast,
}

impl RAst {
    unsafe fn init(mut ptr: NonNull<Self>, ast: Ast) {
        addr_of_mut!(ptr.as_mut()._ast).write(ast)
    }

    unsafe fn _drop(&mut self) {
        addr_of_mut!(self._ast).drop_in_place()
    }

    pub fn new(ast: Ast) -> Result<Ref<Self>, Error> {
        let tp = ast_type().clone();
        unsafe {
            let v = new_gc_obj(size_of::<Self>(), tp)?.cast::<Self>();
            Self::init(v.as_nonnull_ptr(), ast);
            Ok(v)
        }
    }

    pub fn as_ast(&self) -> &Ast {
        &self._ast
    }
}

fn _arith_op_to_opcode(op: ArithOp) -> Option<Opcode> {
    match op {
        ArithOp::Add => Some(Opcode::Add),
        ArithOp::Sub => Some(Opcode::Sub),
        ArithOp::Mul => Some(Opcode::Mul),
        ArithOp::Div => Some(Opcode::Div),
        ArithOp::IDiv => Some(Opcode::IDiv),
        ArithOp::Mod => Some(Opcode::Mod),
        ArithOp::Pow => Some(Opcode::Pow),
        ArithOp::And => Some(Opcode::And),
        ArithOp::Or => Some(Opcode::Or),
        ArithOp::BitAnd => Some(Opcode::BitAnd),
        ArithOp::BitOr => Some(Opcode::BitOr),
        ArithOp::BitXor => Some(Opcode::BitXor),
        ArithOp::Shl => Some(Opcode::Shl),
        ArithOp::Shr => Some(Opcode::Shr),
    }
}

fn _cmp_op_to_opcode(op: CmpOp) -> Option<Opcode> {
    match op {
        CmpOp::Cmp => Some(Opcode::Cmp),
        CmpOp::Eq => Some(Opcode::Eq),
        CmpOp::Ne => Some(Opcode::Ne),
        CmpOp::Lt => Some(Opcode::Lt),
        CmpOp::Le => Some(Opcode::Le),
        CmpOp::Gt => Some(Opcode::Gt),
        CmpOp::Ge => Some(Opcode::Ge),
    }
}

fn _arith_ast_as_code(
    builder: &mut ScriptCodeBuilder,
    op: ArithOp,
    left: &Ref<RAst>,
    right: &Ref<RAst>,
) -> Result<usize, Error> {
    let opcode =
        _arith_op_to_opcode(op).ok_or_else(|| runtime_error_fmt!("invalid arith op: {:?}", op))?;

    let n = _ast_as_code(builder, true, left)?;
    builder.balance_stack(n, 1)?;
    let n = _ast_as_code(builder, true, right)?;
    builder.balance_stack(n, 1)?;

    builder.with_opcode(opcode)?;

    Ok(1)
}

fn _cmp_ast_as_code(
    builder: &mut ScriptCodeBuilder,
    op: CmpOp,
    left: &Ref<RAst>,
    right: &Ref<RAst>,
) -> Result<usize, Error> {
    let opcode =
        _cmp_op_to_opcode(op).ok_or_else(|| runtime_error_fmt!("invalid cmp op: {:?}", op))?;

    let n = _ast_as_code(builder, true, left)?;
    builder.balance_stack(n, 1)?;
    let n = _ast_as_code(builder, true, right)?;
    builder.balance_stack(n, 1)?;

    builder.with_opcode(opcode)?;

    Ok(1)
}

fn _unary_ast_as_code(
    builder: &mut ScriptCodeBuilder,
    op: UnaryOp,
    expr: &Ref<RAst>,
) -> Result<usize, Error> {
    let opcode = match op {
        UnaryOp::Not => Opcode::Nop,
        UnaryOp::BitNot => Opcode::BitAnd,
    };

    let n = _ast_as_code(builder, true, expr)?;
    builder.balance_stack(n, 1)?;

    builder.with_opcode(opcode)?;

    Ok(1)
}

fn _lambda_ast_as_code(
    builder: &mut ScriptCodeBuilder,
    paramets: &Array<Ref<RString>>,
    body: &Ref<RAst>,
) -> Result<usize, Error> {
    let mut new_builder = ScriptCodeBuilder::new(Some(builder))?;

    for name in paramets.as_slice() {
        new_builder.with_paramet(name)?;
    }

    let n = _ast_as_code(&mut new_builder, true, body)?;
    new_builder.balance_stack(n, 1)?;
    new_builder.with_opcode(Opcode::Return)?;

    let code = new_builder.build()?;

    let child_idx = builder.with_child(code.clone())?;

    let mut caps = Array::new(allocator());
    for (name, idx) in code.captured_iter() {
        caps.push((name, idx)).map_err(|_| Error::OutOfMemory)?
    }
    caps.as_slice_mut().sort_by_key(|k| k.1);

    for (name, _idx) in caps.as_slice() {
        if builder.has_local(name) {
            let idx = builder.with_local(name)?;
            builder.with_opcode(Opcode::GetLocal(idx))?;
        } else if builder.has_captured(name) {
            let idx = builder.with_captured(name)?;
            builder.with_opcode(Opcode::GetCapture(idx))?;
        } else {
            return Err(runtime_error_fmt!(
                "invalid captured var: {}",
                name.as_str()
            ));
        }
    }

    builder.with_opcode(Opcode::NewArray(caps.len() as u32))?;

    builder.with_opcode(Opcode::NewClosure(child_idx as u32))?;

    Ok(1)
}

#[allow(non_snake_case)]
fn __capture_collect_sort(
    iter: impl Iterator<Item = (Ref<RString>, u32)>,
    size_hit: Option<usize>,
) -> Result<Array<(Ref<RString>, u32)>, Error> {
    let mut captureds = Array::new(allocator());
    if let Some(size) = size_hit {
        captureds
            .reserve(size)
            .map_err(|_| Error::new_outofmemory())?;
    }
    for (name, idx) in iter {
        captureds
            .push((name, idx))
            .map_err(|_| Error::OutOfMemory)?
    }
    captureds.as_slice_mut().sort_by_key(|k| k.1);
    Ok(captureds)
}

fn _function_def_ast_as_code(
    parent_builder: &mut ScriptCodeBuilder,
    request_value: bool,
    function_name: &Ref<RString>,
    paramets: &Array<Ref<RString>>,
    body: &Ref<RAst>,
) -> Result<usize, Error> {
    let local_idx = parent_builder.with_local(function_name)?;

    let mut func_builder = ScriptCodeBuilder::new(Some(parent_builder))?;

    for name in paramets.as_slice() {
        func_builder.with_paramet(name)?;
    }

    let n = _ast_as_code(&mut func_builder, true, body)?;
    func_builder.balance_stack(n, 1)?;
    func_builder.with_opcode(Opcode::Return)?;

    let capture_self_idx = if func_builder.has_captured(function_name) {
        Some(func_builder.with_captured(function_name)?)
    } else {
        None
    };

    let code = func_builder.build()?;
    let code_idx = parent_builder.with_child(code.clone())?;

    // 用Code对象生成闭包。
    {
        let captureds = __capture_collect_sort(code.captured_iter(), Some(code.children_count()))?;

        for (name, _idx) in captureds.as_slice() {
            if parent_builder.has_local(name) {
                let idx = parent_builder.with_local(name)?;
                parent_builder.with_opcode(Opcode::GetLocal(idx))?;
            } else if parent_builder.has_captured(name) {
                let idx = parent_builder.with_captured(name)?;
                parent_builder.with_opcode(Opcode::GetCapture(idx))?;
            } else {
                return Err(runtime_error_fmt!(
                    "invalid captured var: {}",
                    name.as_str()
                ));
            }
        }

        parent_builder.with_opcode(Opcode::NewArray(captureds.len() as u32))?;
        parent_builder.with_opcode(Opcode::NewClosure(code_idx as u32))?;

        if let Some(cap_idx) = capture_self_idx {
            parent_builder.with_opcode(Opcode::Dup)?;
            parent_builder.with_opcode(Opcode::Dup)?;
            parent_builder.with_opcode(Opcode::SetCapture(cap_idx))?;
        }
    }

    if request_value {
        parent_builder.with_opcode(Opcode::Dup)?;
        parent_builder.with_opcode(Opcode::SetLocal(local_idx))?;
        Ok(1)
    } else {
        parent_builder.with_opcode(Opcode::SetLocal(local_idx))?;
        Ok(0)
    }
}

fn _type_def_ast_as_code(
    parent_builder: &mut ScriptCodeBuilder,
    request_value: bool,
    type_name: &Ref<RString>,
    stats: &Array<Ref<RAst>>,
) -> Result<usize, Error> {
    let type_local_idx = parent_builder.with_local(type_name)?;

    let type_func_code = {
        let mut type_builder = ScriptCodeBuilder::new(Some(parent_builder))?;

        for stat in stats.as_slice() {
            let n = _ast_as_code(&mut type_builder, false, stat)?;
            type_builder.balance_stack(n, 0)?;
        }

        type_builder.with_opcode(Opcode::LoadThis)?;
        type_builder.with_opcode(Opcode::Return)?;

        type_builder.build()?
    };

    let code_idx = parent_builder.with_child(type_func_code.clone())?;

    let type_name_c_idx = parent_builder.with_string(type_name)?;
    parent_builder.with_opcode(Opcode::LoadConstStr(type_name_c_idx as u32))?;
    parent_builder.with_opcode(Opcode::NewType)?;

    parent_builder.with_opcode(Opcode::Dup)?;
    parent_builder.with_opcode(Opcode::SetLocal(type_local_idx))?;

    // 用Code对象生成闭包。
    {
        let captureds = __capture_collect_sort(
            type_func_code.captured_iter(),
            Some(type_func_code.children_count()),
        )?;

        for (name, _idx) in captureds.as_slice() {
            if parent_builder.has_local(name) {
                let idx = parent_builder.with_local(name)?;
                parent_builder.with_opcode(Opcode::GetLocal(idx))?;
            } else if parent_builder.has_captured(name) {
                let idx = parent_builder.with_captured(name)?;
                parent_builder.with_opcode(Opcode::GetCapture(idx))?;
            } else {
                return Err(runtime_error_fmt!(
                    "invalid captured var: {}",
                    name.as_str()
                ));
            }
        }

        parent_builder.with_opcode(Opcode::NewArray(captureds.len() as u32))?;
        parent_builder.with_opcode(Opcode::NewClosure(code_idx as u32))?;
    }

    parent_builder.with_opcode(Opcode::CallThis(0))?;
    parent_builder.with_opcode(Opcode::Pop)?;

    if request_value {
        parent_builder.with_opcode(Opcode::GetLocal(type_local_idx))?;
        Ok(1)
    } else {
        Ok(0)
    }
}

fn _type_public_ast_as_code(
    builder: &mut ScriptCodeBuilder,
    name: &Ref<RString>,
    expr: &Ref<RAst>,
) -> Result<usize, Error> {
    builder.with_opcode(Opcode::LoadThis)?;

    let n = _ast_as_code(builder, true, expr)?;
    builder.balance_stack(n, 1)?;

    let name_c_idx = builder.with_string(name)?;
    builder.with_opcode(Opcode::SetAttr(name_c_idx as u32))?;

    Ok(0)
}

fn _overload_def_ast_as_code(
    parent_builder: &mut ScriptCodeBuilder,
    op: OverloadOp,
    paramets: &Array<Ref<RString>>,
    body: &Ref<RAst>,
) -> Result<usize, Error> {
    let mut func_builder = ScriptCodeBuilder::new(Some(parent_builder))?;

    for name in paramets.as_slice() {
        func_builder.with_paramet(name)?;
    }

    let n = _ast_as_code(&mut func_builder, true, body)?;
    func_builder.balance_stack(n, 1)?;
    func_builder.with_opcode(Opcode::Return)?;

    let code = func_builder.build()?;
    let code_idx = parent_builder.with_child(code.clone())?;

    parent_builder.with_opcode(Opcode::LoadThis)?;

    // 用Code对象生成闭包。
    {
        let captureds = __capture_collect_sort(code.captured_iter(), Some(code.children_count()))?;

        for (name, _idx) in captureds.as_slice() {
            if parent_builder.has_local(name) {
                let idx = parent_builder.with_local(name)?;
                parent_builder.with_opcode(Opcode::GetLocal(idx))?;
            } else if parent_builder.has_captured(name) {
                let idx = parent_builder.with_captured(name)?;
                parent_builder.with_opcode(Opcode::GetCapture(idx))?;
            } else {
                return Err(runtime_error_fmt!(
                    "invalid captured var: {}",
                    name.as_str()
                ));
            }
        }

        parent_builder.with_opcode(Opcode::NewArray(captureds.len() as u32))?;
        parent_builder.with_opcode(Opcode::NewClosure(code_idx as u32))?;
    }

    parent_builder.with_opcode(Opcode::SetOverload(op as u8))?;

    Ok(0)
}

fn _if_ast_as_code(
    builder: &mut ScriptCodeBuilder,
    is_expr: bool,
    cond: &Ref<RAst>,
    truebody: &Ref<RAst>,
    falsebody: &Option<Ref<RAst>>,
) -> Result<usize, Error> {
    builder.with_if(
        |builder| {
            let n = _ast_as_code(builder, true, cond)?;
            builder.balance_stack(n, 1)?;
            Ok(())
        },
        |builder| {
            let n = _ast_as_code(builder, true, truebody)?;
            builder.balance_stack(n, if is_expr { 1 } else { 0 })?;
            Ok(())
        },
        |builder| {
            if let Some(falsebody) = falsebody {
                let n = _ast_as_code(builder, true, falsebody)?;
                builder.balance_stack(n, if is_expr { 1 } else { 0 })?;
            } else {
                if is_expr {
                    builder.with_opcode(Opcode::LoadNull)?;
                }
            }
            Ok(())
        },
    )?;

    Ok(if is_expr { 1 } else { 0 })
}

fn _while_ast_as_code(
    builder: &mut ScriptCodeBuilder,
    _is_expr: bool,
    cond: &Ref<RAst>,
    body: &Ref<RAst>,
) -> Result<usize, Error> {
    builder.with_while_loop(
        |builder| {
            let n = _ast_as_code(builder, true, cond)?;
            builder.balance_stack(n, 1)?;
            Ok(())
        },
        |builder| {
            let n = _ast_as_code(builder, true, body)?;
            builder.balance_stack(n, 0)?;
            Ok(())
        },
    )?;
    Ok(0)
}

fn _for_ast_as_code(
    builder: &mut ScriptCodeBuilder,
    _is_expr: bool,
    name: &Ref<RString>,
    expr: &Ref<RAst>,
    body: &Ref<RAst>,
) -> Result<usize, Error> {
    let name_l_idx = builder.with_local(name)?;
    builder.with_for_loop(
        |builder| {
            let n = _ast_as_code(builder, true, expr)?;
            builder.balance_stack(n, 1)?;
            builder.with_opcode(Opcode::Iter)?;
            Ok(())
        },
        |builder| {
            builder.with_opcode(Opcode::SetLocal(name_l_idx))?;
            let n = _ast_as_code(builder, true, body)?;
            builder.balance_stack(n, 0)?;
            Ok(())
        },
    )?;
    Ok(0)
}

fn _ident_ast_as_code(
    builder: &mut ScriptCodeBuilder,
    name: &Ref<RString>,
) -> Result<usize, Error> {
    if name.as_str() == "this" {
        builder.with_opcode(Opcode::LoadThis)?;
    } else if builder.has_local(&name) {
        let idx = builder.with_local(&name)?;
        builder.with_opcode(Opcode::GetLocal(idx))?;
    } else if let Some(idx) = builder.with_captured_parent(&name)? {
        builder.with_opcode(Opcode::GetCapture(idx))?;
    } else {
        let idx = builder.with_string(&name)?;
        builder.with_opcode(Opcode::GetGlobal(idx as u32))?;
    }

    Ok(1)
}

fn _assign_ast_as_code(
    builder: &mut ScriptCodeBuilder,
    request_expr: bool,
    target: &Ref<RAst>,
    expr: &Ref<RAst>,
) -> Result<usize, Error> {
    if let Ast::Ident { name } = target.as_ast() {
        if name.as_str() == "this" {
            return Err(runtime_error_fmt!("cannot assign a value to \"this\""));
        }

        let idx = builder.with_local(name)?;
        let n = _ast_as_code(builder, true, expr)?;
        builder.balance_stack(n, 1)?;
        if request_expr {
            builder.with_opcode(Opcode::Dup)?;
            builder.with_opcode(Opcode::SetLocal(idx))?;
            Ok(1)
        } else {
            builder.with_opcode(Opcode::SetLocal(idx))?;
            Ok(0)
        }
    } else if let Ast::Attr {
        expr: target_expr,
        name,
    } = target.as_ast()
    {
        let n = _ast_as_code(builder, true, target_expr)?;
        builder.balance_stack(n, 1)?;
        let n = _ast_as_code(builder, true, expr)?;
        builder.balance_stack(n, 1)?;

        let name_c_idx = builder.with_string(name)?;

        if request_expr {
            builder.with_opcode(Opcode::Dup)?;
            builder.with_opcode(Opcode::Rot3)?;
            builder.with_opcode(Opcode::SetAttr(name_c_idx as u32))?;
            Ok(1)
        } else {
            builder.with_opcode(Opcode::SetAttr(name_c_idx as u32))?;
            Ok(0)
        }
    } else if let Ast::Index {
        expr: target_expr,
        index: index_expr,
    } = target.as_ast()
    {
        let n = _ast_as_code(builder, true, target_expr)?;
        builder.balance_stack(n, 1)?;
        let n = _ast_as_code(builder, true, index_expr)?;
        builder.balance_stack(n, 1)?;
        let n = _ast_as_code(builder, true, expr)?;
        builder.balance_stack(n, 1)?;
        if request_expr {
            builder.with_opcode(Opcode::Dup)?;
            builder.with_opcode(Opcode::Rot4)?;
            builder.with_opcode(Opcode::SetItem)?;
            Ok(1)
        } else {
            builder.with_opcode(Opcode::SetItem)?;
            Ok(0)
        }
    } else {
        Err(runtime_error_fmt!(
            "the left side of the assignor must be an assignable expression"
        ))
    }
}

fn _attr_ast_as_code(
    builder: &mut ScriptCodeBuilder,
    expr: &Ref<RAst>,
    name: &Ref<RString>,
    dup: bool,
) -> Result<usize, Error> {
    let n = _ast_as_code(builder, true, expr)?;
    builder.balance_stack(n, 1)?;

    let name_c_idx = builder.with_string(&name)?;
    if dup {
        builder.with_opcode(Opcode::GetAttrDup(name_c_idx as u32))?;
        Ok(2)
    } else {
        builder.with_opcode(Opcode::GetAttr(name_c_idx as u32))?;
        Ok(1)
    }
}

fn _index_ast_as_code(
    builder: &mut ScriptCodeBuilder,
    expr: &Ref<RAst>,
    index: &Ref<RAst>,
) -> Result<usize, Error> {
    let n = _ast_as_code(builder, true, expr)?;
    builder.balance_stack(n, 1)?;

    let n = _ast_as_code(builder, true, index)?;
    builder.balance_stack(n, 1)?;

    builder.with_opcode(Opcode::GetItem)?;
    Ok(1)
}

fn _call_ast_as_code(
    builder: &mut ScriptCodeBuilder,
    func: &Ref<RAst>,
    args: &Array<Ref<RAst>>,
) -> Result<usize, Error> {
    let n = _ast_as_code(builder, true, func)?;
    builder.balance_stack(n, 1)?;

    for arg in args.as_slice() {
        let n = _ast_as_code(builder, true, arg)?;
        builder.balance_stack(n, 1)?;
    }

    builder.with_opcode(Opcode::Call(args.len() as u32))?;

    Ok(1)
}

fn _method_call_ast_as_code(
    builder: &mut ScriptCodeBuilder,
    target: &Ref<RAst>,
    name: &Ref<RString>,
    args: &Array<Ref<RAst>>,
) -> Result<usize, Error> {
    let n = _ast_as_code(builder, true, target)?;
    builder.balance_stack(n, 1)?;

    for arg in args.as_slice() {
        let n = _ast_as_code(builder, true, arg)?;
        builder.balance_stack(n, 1)?;
    }

    let name_c_idx = builder.with_string(&name)?;
    // TODO: 检查name_c_idx的范围是否超出u16。
    builder.with_opcode(Opcode::CallMethod(name_c_idx as u16, args.len() as u16))?;

    Ok(1)
}

fn _attr_call_ast_as_code(
    builder: &mut ScriptCodeBuilder,
    target: &Ref<RAst>,
    name: &Ref<RString>,
    args: &Array<Ref<RAst>>,
) -> Result<usize, Error> {
    let n = _ast_as_code(builder, true, target)?;
    builder.balance_stack(n, 1)?;

    for arg in args.as_slice() {
        let n = _ast_as_code(builder, true, arg)?;
        builder.balance_stack(n, 1)?;
    }

    let name_c_idx = builder.with_string(&name)?;
    // TODO: 检查name_c_idx的范围是否超出u16。
    builder.with_opcode(Opcode::CallAttr(name_c_idx as u16, args.len() as u16))?;

    Ok(1)
}

fn _stat_ast_as_code(
    builder: &mut ScriptCodeBuilder,
    expr: &Option<Ref<RAst>>,
) -> Result<usize, Error> {
    if let Some(expr) = expr {
        _ast_as_code(builder, false, expr)
    } else {
        Ok(0)
    }
}

fn _program_public_ast_as_code(
    builder: &mut ScriptCodeBuilder,
    name: &Ref<RString>,
    expr: &Ref<RAst>,
) -> Result<usize, Error> {
    builder.with_opcode(Opcode::LoadThis)?;

    let n = _ast_as_code(builder, true, expr)?;
    builder.balance_stack(n, 1)?;

    let name_c_idx = builder.with_string(name)?;
    builder.with_opcode(Opcode::SetAttr(name_c_idx as u32))?;

    Ok(0)
}

fn _ast_as_code(
    builder: &mut ScriptCodeBuilder,
    request_value: bool,
    ast: &Ref<RAst>,
) -> Result<usize, Error> {
    match ast.as_ast() {
        Ast::Int(n) => {
            if *n > (i32::MAX as Int) {
                let int_c_idx = builder.with_integer(*n)?;
                builder.with_opcode(Opcode::LoadConstNum(int_c_idx as u32))?;
            } else {
                builder.with_opcode(Opcode::LoadInt(*n as i32))?;
            }
            Ok(1)
        }
        Ast::Float(n) => {
            let float_c_idx = builder.with_float(*n)?;
            builder.with_opcode(Opcode::LoadConstNum(float_c_idx as u32))?;
            Ok(1)
        }
        Ast::String(s) => {
            let str_c_idx = builder.with_string(s)?;
            builder.with_opcode(Opcode::LoadConstStr(str_c_idx as u32))?;
            Ok(1)
        }
        Ast::Tuple(arr) => {
            let arr_len = arr.len();
            if arr_len > (u32::MAX as usize) {
                return Err(runtime_error_fmt!("too many tuple items"));
            }
            for ast in arr.as_slice() {
                let n = _ast_as_code(builder, true, ast)?;
                builder.balance_stack(n, 1)?;
            }
            builder.with_opcode(Opcode::NewTuple(arr_len as u32))?;
            Ok(1)
        }
        Ast::Array(arr) => {
            let arr_len = arr.len();
            if arr_len > (u32::MAX as usize) {
                return Err(runtime_error_fmt!("too many array items"));
            }
            for ast in arr.as_slice() {
                let n = _ast_as_code(builder, true, ast)?;
                builder.balance_stack(n, 1)?;
            }
            builder.with_opcode(Opcode::NewArray(arr_len as u32))?;
            Ok(1)
        }
        Ast::Map(map) => {
            let map_len = map.len();
            if map_len > (u32::MAX as usize) {
                return Err(runtime_error_fmt!("too many map items"));
            }
            for (key_ast, value_ast) in map.as_slice() {
                let n = _ast_as_code(builder, true, key_ast)?;
                builder.balance_stack(n, 1)?;
                let n = _ast_as_code(builder, true, value_ast)?;
                builder.balance_stack(n, 1)?;
            }
            builder.with_opcode(Opcode::NewMap(map_len as u32))?;
            Ok(1)
        }
        Ast::Program { stats: _, expr: _ } => Err(runtime_error_fmt!(
            "Ast::Program can only appear at the top level"
        )),
        Ast::ProgramPublic { name, expr } => {
            let n = _program_public_ast_as_code(builder, name, expr)?;
            Ok(n)
        }
        Ast::Block { stats, expr } => {
            for ast in stats.as_slice() {
                let n = _ast_as_code(builder, false, ast)?;
                builder.balance_stack(n, 0)?;
            }
            if let Some(expr) = expr {
                let n = _ast_as_code(builder, true, expr)?;
                Ok(n)
            } else {
                Ok(0)
            }
        }
        Ast::ArithExpr { op, left, right } => {
            let n = _arith_ast_as_code(builder, *op, left, right)?;
            Ok(n)
        }
        Ast::CmpExpr { op, left, right } => {
            let n = _cmp_ast_as_code(builder, *op, left, right)?;
            Ok(n)
        }
        Ast::UnaryExpr { op, expr } => {
            let n = _unary_ast_as_code(builder, *op, expr)?;
            Ok(n)
        }
        Ast::Lambda { paramets, body } => {
            let n = _lambda_ast_as_code(builder, paramets, body)?;
            Ok(n)
        }
        Ast::FunctionDef {
            name,
            paramets,
            body,
        } => {
            let n = _function_def_ast_as_code(builder, request_value, name, paramets, body)?;
            Ok(n)
        }
        Ast::TypeDef { name, stats } => {
            let n = _type_def_ast_as_code(builder, request_value, name, stats)?;
            Ok(n)
        }
        Ast::TypePublic { name, expr } => {
            let n = _type_public_ast_as_code(builder, name, expr)?;
            Ok(n)
        }
        Ast::OverloadDef { op, paramets, body } => {
            let n = _overload_def_ast_as_code(builder, *op, paramets, body)?;
            Ok(n)
        }
        Ast::If {
            is_expr,
            cond,
            truebody,
            falsebody,
        } => {
            let n = _if_ast_as_code(builder, *is_expr, cond, truebody, falsebody)?;
            Ok(n)
        }
        Ast::While {
            is_expr,
            cond,
            body,
        } => {
            let n = _while_ast_as_code(builder, *is_expr, cond, body)?;
            builder.balance_stack(n, 0)?;
            Ok(0)
        }
        Ast::For {
            is_expr,
            name,
            expr,
            body,
        } => {
            let n = _for_ast_as_code(builder, *is_expr, name, expr, body)?;
            builder.balance_stack(n, 0)?;
            Ok(0)
        }
        Ast::Ident { name } => {
            let n = _ident_ast_as_code(builder, name)?;
            Ok(n)
        }
        Ast::Assign { target, expr } => {
            let n = _assign_ast_as_code(builder, request_value, target, expr)?;
            Ok(n)
        }
        Ast::Attr { expr, name } => {
            let n = _attr_ast_as_code(builder, expr, name, false)?;
            Ok(n)
        }
        Ast::Index { expr, index } => {
            let n = _index_ast_as_code(builder, expr, index)?;
            Ok(n)
        }
        Ast::Call { func, args } => {
            let n = _call_ast_as_code(builder, func, args)?;
            Ok(n)
        }
        Ast::MethodCall {
            target: func,
            name,
            args,
        } => {
            let n = _method_call_ast_as_code(builder, func, name, args)?;
            Ok(n)
        }
        Ast::AttrCall {
            target: func,
            name,
            args,
        } => {
            let n = _attr_call_ast_as_code(builder, func, name, args)?;
            Ok(n)
        }
        Ast::Return { expr } => {
            if let Some(expr) = expr {
                let n = _ast_as_code(builder, true, expr)?;
                builder.balance_stack(n, 1)?;
            } else {
                builder.with_opcode(Opcode::LoadNull)?;
            }
            builder.with_opcode(Opcode::Return)?;
            Ok(0)
        }
        Ast::Stat { expr } => {
            let n = _stat_ast_as_code(builder, expr)?;
            Ok(n)
        }
    }
}

pub(crate) fn ast_as_code(ast: Ref<RAst>, _ret_value: bool) -> Result<Ref<RScriptCode>, Error> {
    if let Ast::Program { stats, expr } = ast.as_ast() {
        let mut builder = ScriptCodeBuilder::new(None)?;

        for stat in stats.as_slice() {
            let n = _ast_as_code(&mut builder, false, stat)?;
            builder.balance_stack(n, 0)?;
        }

        if let Some(expr) = expr {
            let n = _ast_as_code(&mut builder, true, expr)?;
            builder.balance_stack(n, 1)?;
        } else {
            builder.with_opcode(Opcode::LoadNull)?;
        }
        builder.with_opcode(Opcode::Return)?;

        let code = builder.build()?;
        Ok(code)
    } else {
        Err(runtime_error_fmt!("the top AST must be Ast::Program"))
    }
}

pub(crate) fn _init_type_ast(mut tp: Ref<RType>) -> Result<(), Error> {
    tp.with_visit(ast__visit);

    tp.with_destory(ast__destory);

    tp.with_eq(default_value_eq);
    tp.with_hash(default_value_hash);
    tp.with_str(default_value_str);

    Ok(())
}

use crate::runtime::Visitor;

#[allow(non_snake_case)]
fn ast__visit(visitor: &mut dyn Visitor, value_ptr: NonNull<GcHeader>) {
    unsafe {
        let ast = value_ptr.cast::<RAst>().as_ref();

        use Ast::*;
        match ast.as_ast() {
            Int(_) => (),
            Float(_) => (),
            String(v) => visitor.visit_value(v.cast_value_ref()),
            Tuple(arr) => {
                for v in arr.as_slice() {
                    visitor.visit_value(v.cast_value_ref())
                }
            }
            Array(arr) => {
                for v in arr.as_slice() {
                    visitor.visit_value(v.cast_value_ref())
                }
            }
            Map(arr) => {
                for (a, b) in arr.as_slice() {
                    visitor.visit_value(a.cast_value_ref());
                    visitor.visit_value(b.cast_value_ref());
                }
            }
            Program { stats, expr } => {
                for s in stats.as_slice() {
                    visitor.visit_value(s.cast_value_ref());
                }
                if let Some(e) = expr {
                    visitor.visit_value(e.cast_value_ref());
                }
            }
            ProgramPublic { name, expr } => {
                visitor.visit_value(name.cast_value_ref());
                visitor.visit_value(expr.cast_value_ref());
            }
            Block { stats, expr } => {
                for s in stats.as_slice() {
                    visitor.visit_value(s.cast_value_ref());
                }
                if let Some(e) = expr {
                    visitor.visit_value(e.cast_value_ref());
                }
            }
            ArithExpr { op: _, left, right } => {
                visitor.visit_value(left.cast_value_ref());
                visitor.visit_value(right.cast_value_ref());
            }
            CmpExpr { op: _, left, right } => {
                visitor.visit_value(left.cast_value_ref());
                visitor.visit_value(right.cast_value_ref());
            }
            UnaryExpr { op: _, expr } => {
                visitor.visit_value(expr.cast_value_ref());
            }
            Lambda { paramets, body } => {
                for s in paramets.as_slice() {
                    visitor.visit_value(s.cast_value_ref());
                }
                visitor.visit_value(body.cast_value_ref());
            }
            FunctionDef {
                name,
                paramets,
                body,
            } => {
                visitor.visit_value(name.cast_value_ref());
                for s in paramets.as_slice() {
                    visitor.visit_value(s.cast_value_ref());
                }
                visitor.visit_value(body.cast_value_ref());
            }
            TypeDef { name, stats } => {
                visitor.visit_value(name.cast_value_ref());
                for s in stats.as_slice() {
                    visitor.visit_value(s.cast_value_ref());
                }
            }
            OverloadDef {
                op: _,
                paramets,
                body,
            } => {
                for s in paramets.as_slice() {
                    visitor.visit_value(s.cast_value_ref());
                }
                visitor.visit_value(body.cast_value_ref());
            }
            TypePublic { name, expr } => {
                visitor.visit_value(name.cast_value_ref());
                visitor.visit_value(expr.cast_value_ref());
            }
            If {
                is_expr: _,
                cond,
                truebody,
                falsebody,
            } => {
                visitor.visit_value(cond.cast_value_ref());
                visitor.visit_value(truebody.cast_value_ref());
                if let Some(e) = falsebody {
                    visitor.visit_value(e.cast_value_ref());
                }
            }
            While {
                is_expr: _,
                cond,
                body,
            } => {
                visitor.visit_value(cond.cast_value_ref());
                visitor.visit_value(body.cast_value_ref());
            }
            For {
                is_expr: _,
                name,
                expr,
                body,
            } => {
                visitor.visit_value(name.cast_value_ref());
                visitor.visit_value(expr.cast_value_ref());
                visitor.visit_value(body.cast_value_ref());
            }
            Ident { name } => visitor.visit_value(name.cast_value_ref()),
            Assign { target, expr } => {
                visitor.visit_value(target.cast_value_ref());
                visitor.visit_value(expr.cast_value_ref());
            }
            Attr { expr, name } => {
                visitor.visit_value(expr.cast_value_ref());
                visitor.visit_value(name.cast_value_ref());
            }
            Index { expr, index } => {
                visitor.visit_value(expr.cast_value_ref());
                visitor.visit_value(index.cast_value_ref());
            }
            Call { func, args } => {
                visitor.visit_value(func.cast_value_ref());
                for v in args.as_slice() {
                    visitor.visit_value(v.cast_value_ref());
                }
            }
            MethodCall { target, name, args } => {
                visitor.visit_value(target.cast_value_ref());
                visitor.visit_value(name.cast_value_ref());
                for v in args.as_slice() {
                    visitor.visit_value(v.cast_value_ref());
                }
            }
            AttrCall { target, name, args } => {
                visitor.visit_value(target.cast_value_ref());
                visitor.visit_value(name.cast_value_ref());
                for v in args.as_slice() {
                    visitor.visit_value(v.cast_value_ref());
                }
            }
            Return { expr } => {
                if let Some(expr) = expr {
                    visitor.visit_value(expr.cast_value_ref());
                }
            }
            Stat { expr } => {
                if let Some(expr) = expr {
                    visitor.visit_value(expr.cast_value_ref());
                }
            }
        }
    }
}

#[allow(non_snake_case)]
fn ast__destory(value: &RValue) -> Result<(), Error> {
    unsafe {
        let mut ast = value.expect_cast::<RAst>(ast_type())?;
        ast._drop();
        Ok(())
    }
}
