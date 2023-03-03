use core::mem::size_of;
use core::ptr::addr_of_mut;
use core::ptr::NonNull;

use crate::collections::Array;

use crate::runtime::*;

use crate::number::*;
use crate::string::*;
use crate::type_::*;
use crate::value::*;

use crate::op::*;

use crate::error::*;
use crate::runtime_error_fmt;

use crate::builtin::*;

use crate::util::StringMap;

#[repr(C)]
pub struct RScriptCode {
    _header: GcHeader,
    _parent: Option<Ref<RScriptCode>>,
    _paramet_count: u32,
    _variable: bool,
    _opcodes: Array<Opcode>,
    _chlidren: Array<Ref<RScriptCode>>,
    _strings: Array<Ref<RString>>,
    _numbers: Array<RValue>,
    _captured_vars: StringMap<u32>,
    _local_vars: StringMap<u32>,
}

impl RScriptCode {
    unsafe fn init(mut ptr: NonNull<Self>) {
        let allocator = allocator();
        let r = ptr.as_mut();
        addr_of_mut!(r._parent).write(None);
        addr_of_mut!(r._paramet_count).write(0);
        addr_of_mut!(r._variable).write(false);
        addr_of_mut!(r._opcodes).write(Array::new(allocator));
        addr_of_mut!(r._chlidren).write(Array::new(allocator));
        addr_of_mut!(r._strings).write(Array::new(allocator));
        addr_of_mut!(r._numbers).write(Array::new(allocator));
        addr_of_mut!(r._captured_vars).write(StringMap::new(allocator));
        addr_of_mut!(r._local_vars).write(StringMap::new(allocator));
    }

    unsafe fn _drop(&mut self) {
        addr_of_mut!(self._parent).drop_in_place();
        addr_of_mut!(self._opcodes).drop_in_place();
        addr_of_mut!(self._chlidren).drop_in_place();
        addr_of_mut!(self._strings).drop_in_place();
        addr_of_mut!(self._numbers).drop_in_place();
        addr_of_mut!(self._captured_vars).drop_in_place();
        addr_of_mut!(self._local_vars).drop_in_place();
    }

    pub(self) fn new() -> Result<Ref<Self>, Error> {
        let tp = script_code_type().clone();
        unsafe {
            let v = new_gc_obj(size_of::<Self>(), tp)?.cast::<Self>();
            Self::init(v.as_nonnull_ptr());
            Ok(v)
        }
    }

    pub fn parent(&self) -> Option<Ref<RScriptCode>> {
        self._parent.clone()
    }

    pub fn paramet_count(&self) -> u32 {
        self._paramet_count
    }

    pub fn is_variable(&self) -> bool {
        self._variable
    }

    pub fn opcode(&self) -> &[Opcode] {
        self._opcodes.as_slice()
    }

    pub fn children_count(&self) -> usize {
        self._chlidren.len()
    }

    pub fn get_child(&self, n: usize) -> Option<Ref<RScriptCode>> {
        self._chlidren.get(n).cloned()
    }

    pub fn captured_count(&self) -> usize {
        self._captured_vars.len()
    }

    pub fn local_count(&self) -> usize {
        self._local_vars.len()
    }

    pub fn captured_iter(&self) -> impl Iterator<Item = (Ref<RString>, u32)> + '_ {
        self._captured_vars.iter().map(|(k, v)| (k.clone(), *v))
    }

    pub fn local_iter(&self) -> impl Iterator<Item = (Ref<RString>, u32)> + '_ {
        self._local_vars.iter().map(|(k, v)| (k.clone(), *v))
    }

    pub fn get_const_string(&self, idx: usize) -> Option<Ref<RString>> {
        self._strings.get(idx).cloned()
    }
    pub fn get_const_number(&self, idx: usize) -> Option<RValue> {
        self._numbers.get(idx).cloned()
    }
}

pub(crate) fn _init_type_script_code(mut tp: Ref<RType>) -> Result<(), Error> {
    tp.with_visit(script_code__visit);

    tp.with_destory(script_code__destory);

    tp.with_eq(default_value_eq);
    tp.with_hash(default_value_hash);
    tp.with_str(default_value_str);

    Ok(())
}

use crate::runtime::Visitor;

#[allow(non_snake_case)]
fn script_code__visit(visitor: &mut dyn Visitor, value_ptr: NonNull<GcHeader>) {
    unsafe {
        let code = value_ptr.cast::<RScriptCode>().as_ref();

        if let Some(parent) = &code._parent {
            visitor.visit_value(parent.cast_value_ref());
        }

        for child in code._chlidren.as_slice() {
            visitor.visit_value(child.cast_value_ref());
        }

        for s in code._strings.as_slice() {
            visitor.visit_value(s.cast_value_ref());
        }
        for n in code._numbers.as_slice() {
            visitor.visit_value(n.cast_value_ref());
        }

        for (k, _) in code._captured_vars.iter() {
            visitor.visit_value(k.cast_value_ref());
        }
        for (k, _) in code._local_vars.iter() {
            visitor.visit_value(k.cast_value_ref());
        }
    }
}

#[allow(non_snake_case)]
fn script_code__destory(value: &RValue) -> Result<(), Error> {
    unsafe {
        let mut sc = value.expect_cast::<RScriptCode>(script_code_type())?;
        sc._drop();
        Ok(())
    }
}

use crate::collections::HashMap;
use core::hash::{Hash, Hasher};

#[derive(Clone, Copy, Debug)]
enum Number {
    Int(Int),
    Float(Float),
}
impl Hash for Number {
    fn hash<H: Hasher>(&self, state: &mut H) {
        use core::mem::transmute;
        match self {
            Self::Int(n) => state.write_isize(*n),
            Self::Float(n) => unsafe { state.write_isize(transmute(*n)) },
        }
    }
}
impl Eq for Number {}
impl PartialEq for Number {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Int(a), Self::Int(b)) if a == b => true,
            (Self::Float(a), Self::Float(b)) if a == b => true,
            _ => false,
        }
    }
}

pub struct ScriptCodeBuilder {
    _parent: Option<*mut ScriptCodeBuilder>,
    _code: Ref<RScriptCode>,
    _string_idxs: StringMap<usize>,
    _strings: Array<Ref<RString>>,
    _number_idxs: HashMap<Number, usize>,
    _numbers: Array<RValue>,
    _labels: Array<usize>,
}

#[allow(dead_code)]
impl ScriptCodeBuilder {
    pub fn new(parent: Option<&mut ScriptCodeBuilder>) -> Result<Self, Error> {
        let code = RScriptCode::new()?;
        let allocator = allocator();
        let builder = Self {
            _parent: parent.map(|v| v as *mut _),
            _code: code,
            _string_idxs: StringMap::new(allocator),
            _strings: Array::new(allocator),
            _number_idxs: HashMap::new(allocator),
            _numbers: Array::new(allocator),
            _labels: Array::new(allocator),
        };
        Ok(builder)
    }

    fn parent<'a, 'b: 'a>(&'a mut self) -> Option<&'b mut Self> {
        unsafe { self._parent.map(|v| &mut *v) }
    }

    pub fn with_variable(&mut self, var: bool) {
        self._code._variable = var;
    }

    // create a label, return the label id.
    pub fn with_label(&mut self, op_pos: usize) -> Result<u32, Error> {
        let id = self._labels.len();
        self._labels.push(op_pos).map_err(|_| Error::OutOfMemory)?;
        Ok(id as u32)
    }

    #[allow(unused_must_use)]
    pub fn set_label(&mut self, label: u32, op_pos: usize) {
        if (label as usize) < self._labels.len() {
            self._labels.set(label as usize, op_pos);
        }
    }

    pub fn with_opcode(&mut self, opcode: Opcode) -> Result<usize, Error> {
        self._code
            ._opcodes
            .push(opcode)
            .map_err(|_| Error::OutOfMemory)?;
        Ok(self._code._opcodes.len() - 1)
    }

    pub fn remove_opcode(&mut self, idx: usize) {
        if idx < self._code._opcodes.len() {
            self._code._opcodes.remove(idx);
        }
    }

    pub fn with_if<CF, TF, FF>(
        &mut self,
        cond: CF,
        truebody: TF,
        falsebody: FF,
    ) -> Result<(), Error>
    where
        CF: FnOnce(&mut Self) -> Result<(), Error>,
        TF: FnOnce(&mut Self) -> Result<(), Error>,
        FF: FnOnce(&mut Self) -> Result<(), Error>,
    {
        cond(self)?;
        let if_opcode_pos = self.with_opcode(Opcode::Nop)?;
        truebody(self)?;
        let jmp_opcode_pos = self.with_opcode(Opcode::Nop)?;
        let falseody_start_label = self.with_label(self.current_opcode_pos())?;
        falsebody(self)?;
        let falsebody_end_label = self.with_label(self.current_opcode_pos())?;
        self.replace_opcode(if_opcode_pos, Opcode::IfFalseLabel(falseody_start_label));
        self.replace_opcode(jmp_opcode_pos, Opcode::JmpLabel(falsebody_end_label));
        Ok(())
    }

    pub fn with_while_loop<CF, BF>(&mut self, cond: CF, body: BF) -> Result<(), Error>
    where
        CF: FnOnce(&mut Self) -> Result<(), Error>,
        BF: FnOnce(&mut Self) -> Result<(), Error>,
    {
        let cond_start_label = self.with_label(self.current_opcode_pos())?;
        cond(self)?;
        let if_opcode_pos = self.with_opcode(Opcode::Nop)?;

        body(self)?;
        let jmp_opcode_pos = self.with_opcode(Opcode::Nop)?;
        let body_end_label = self.with_label(self.current_opcode_pos())?;

        self.replace_opcode(if_opcode_pos, Opcode::IfFalseLabel(body_end_label));
        self.replace_opcode(jmp_opcode_pos, Opcode::JmpLabel(cond_start_label));
        Ok(())
    }

    pub fn with_for_loop<EF, BF>(&mut self, iterable_expr: EF, body: BF) -> Result<(), Error>
    where
        EF: FnOnce(&mut Self) -> Result<(), Error>,
        BF: FnOnce(&mut Self) -> Result<(), Error>,
    {
        iterable_expr(self)?;

        let cond_start_label = self.with_label(self.current_opcode_pos())?;
        self.with_opcode(Opcode::Dup)?;
        let cond_opcode_pos = self.with_opcode(Opcode::Nop)?;

        body(self)?;
        let jmp_opcode_pos = self.with_opcode(Opcode::Nop)?;
        let body_end_label = self.with_label(self.current_opcode_pos())?;

        self.with_opcode(Opcode::Pop)?;

        self.replace_opcode(cond_opcode_pos, Opcode::IterNextLabel(body_end_label));
        self.replace_opcode(jmp_opcode_pos, Opcode::JmpLabel(cond_start_label));
        Ok(())
    }

    pub fn current_opcode_pos(&self) -> usize {
        self._code._opcodes.len()
    }
    pub fn last_opcode_pos(&self) -> usize {
        self._code._opcodes.len() - 1
    }

    pub fn get_opcode(&self, pos: usize) -> Option<Opcode> {
        self._code._opcodes.get(pos).cloned()
    }

    #[allow(unused_must_use)]
    pub fn replace_opcode(&mut self, idx: usize, opcode: Opcode) {
        if idx < self._code._opcodes.len() {
            self._code._opcodes.set(idx, opcode);
        }
    }

    pub fn with_child(&mut self, code: Ref<RScriptCode>) -> Result<usize, Error> {
        self._code
            ._chlidren
            .push(code)
            .map_err(|_| Error::OutOfMemory)?;
        Ok(self._code._chlidren.len() - 1)
    }

    pub fn with_paramet(&mut self, name: &Ref<RString>) -> Result<(), Error> {
        self.with_local(name)?;
        self._code._paramet_count += 1;
        Ok(())
    }

    pub fn with_captured(&mut self, name: &Ref<RString>) -> Result<u32, Error> {
        let res = self._code._captured_vars.get(name).cloned();
        if let Some(n) = res {
            Ok(n)
        } else {
            let n = self._code._captured_vars.len() as u32;
            self._code
                ._captured_vars
                .insert(name.clone(), n)
                .map_err(|_| Error::OutOfMemory)?;
            Ok(n)
        }
    }

    pub fn with_captured_parent(&mut self, name: &Ref<RString>) -> Result<Option<u32>, Error> {
        if let Some(p) = self.parent().take() {
            let res = if p.has_local(name) {
                self.with_captured(name).map(|v| Some(v))
            } else if p.has_captured(name) {
                self.with_captured(name).map(|v| Some(v))
            } else {
                if let Some(_idx) = p.with_captured_parent(name)? {
                    self.with_captured(name).map(|v| Some(v))
                } else {
                    Ok(None)
                }
            };
            self._parent = Some(p);
            res
        } else {
            Ok(None)
        }
    }

    pub fn with_local(&mut self, name: &Ref<RString>) -> Result<u32, Error> {
        let res = self._code._local_vars.get(name).cloned();
        if let Some(n) = res {
            Ok(n)
        } else {
            let n = self._code._local_vars.len() as u32;
            self._code
                ._local_vars
                .insert(name.clone(), n)
                .map_err(|_| Error::OutOfMemory)?;
            Ok(n)
        }
    }

    pub fn with_string(&mut self, string: &Ref<RString>) -> Result<usize, Error> {
        if let Some(idx) = self._string_idxs.get(string) {
            Ok(*idx)
        } else {
            let idx = self._strings.len();
            self._strings
                .push(string.clone())
                .map_err(|_| Error::OutOfMemory)?;
            self._string_idxs
                .insert(string.clone(), idx)
                .map_err(|_| Error::OutOfMemory)?;
            Ok(idx)
        }
    }

    pub fn with_integer(&mut self, n: Int) -> Result<usize, Error> {
        let number = Number::Int(n);
        if let Some(idx) = self._number_idxs.get(&number) {
            Ok(*idx)
        } else {
            let idx = self._numbers.len();
            self._numbers
                .push(RInt::new(n)?.cast_value())
                .map_err(|_| Error::OutOfMemory)?;
            self._number_idxs
                .insert(number, idx)
                .map_err(|_| Error::OutOfMemory)?;
            Ok(idx)
        }
    }

    pub fn with_float(&mut self, n: Float) -> Result<usize, Error> {
        let number = Number::Float(n);
        if let Some(idx) = self._number_idxs.get(&number) {
            Ok(*idx)
        } else {
            let idx = self._numbers.len();
            self._numbers
                .push(RFloat::new(n)?.cast_value())
                .map_err(|_| Error::OutOfMemory)?;
            self._number_idxs
                .insert(number, idx)
                .map_err(|_| Error::OutOfMemory)?;
            Ok(idx)
        }
    }

    pub fn has_local(&self, name: &Ref<RString>) -> bool {
        self._code._local_vars.contains_key(name)
    }

    pub fn has_captured(&self, name: &Ref<RString>) -> bool {
        self._code._captured_vars.contains_key(name)
    }

    pub fn has_captured_parent(&self, name: &Ref<RString>) -> bool {
        let mut current = self._code.clone();
        loop {
            if current._captured_vars.contains_key(name) {
                return true;
            }
            if let Some(p) = current.parent() {
                current = p;
            } else {
                break;
            }
        }
        false
    }

    pub fn balance_stack(&mut self, from: usize, to: usize) -> Result<(), Error> {
        if from > to {
            for _ in 0..(from - to) {
                self.with_opcode(Opcode::Pop)
                    .map_err(|_| Error::OutOfMemory)?;
            }
        } else if from < to {
            for _ in 0..(to - from) {
                self.with_opcode(Opcode::LoadNull)
                    .map_err(|_| Error::OutOfMemory)?;
            }
        }
        Ok(())
    }

    pub(crate) fn opcode(&self) -> &[Opcode] {
        self._code.opcode()
    }

    fn _replace_label(&mut self) -> Result<(), Error> {
        let ops = self._code._opcodes.as_slice_mut();
        for i in 0..ops.len() {
            match ops[i] {
                Opcode::IfFalseLabel(n) => {
                    let pos = self._labels.get(n as usize).ok_or_else(|| {
                        runtime_error_fmt!("in ScriptFunciton build, invalid label")
                    })?;
                    let offset = (*pos as i32) - (i as i32);
                    ops[i] = Opcode::IfFalse(offset);
                }
                Opcode::JmpLabel(n) => {
                    let pos = self._labels.get(n as usize).ok_or_else(|| {
                        runtime_error_fmt!("in ScriptFunciton build, invalid label")
                    })?;
                    let offset = (*pos as i32) - (i as i32);
                    ops[i] = Opcode::Jmp(offset);
                }
                Opcode::IterNextLabel(n) => {
                    let pos = self._labels.get(n as usize).ok_or_else(|| {
                        runtime_error_fmt!("in ScriptFunciton build, invalid label")
                    })?;
                    let offset = (*pos as i32) - (i as i32);
                    ops[i] = Opcode::IterNext(offset);
                }
                _ => {}
            };
        }
        Ok(())
    }

    fn _fill_const(&mut self) -> Result<(), Error> {
        core::mem::swap(&mut self._code._strings, &mut self._strings);
        core::mem::swap(&mut self._code._numbers, &mut self._numbers);
        Ok(())
    }

    pub fn build(mut self) -> Result<Ref<RScriptCode>, Error> {
        self._replace_label()?;
        self._fill_const()?;
        Ok(self._code)
    }
}
