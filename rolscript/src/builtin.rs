use core::mem::MaybeUninit;

use crate::runtime::*;

use crate::error::*;

use crate::ast::_init_type_ast;

use crate::array::*;
use crate::function::*;
use crate::map::*;
use crate::module::*;
use crate::number::*;
use crate::option::*;
use crate::script_code::*;
use crate::string::*;
use crate::tuple::*;
use crate::type_::*;
use crate::value::*;

static mut _TYPE_TYPE_: MaybeUninit<Ref<RType>> = MaybeUninit::uninit();
static mut _NULL_TYPE_: MaybeUninit<Ref<RType>> = MaybeUninit::uninit();
static mut _BOOL_TYPE_: MaybeUninit<Ref<RType>> = MaybeUninit::uninit();
static mut _INT_TYPE_: MaybeUninit<Ref<RType>> = MaybeUninit::uninit();
static mut _FLOAT_TYPE_: MaybeUninit<Ref<RType>> = MaybeUninit::uninit();
static mut _STRING_TYPE_: MaybeUninit<Ref<RType>> = MaybeUninit::uninit();
static mut _TUPLE_TYPE_: MaybeUninit<Ref<RType>> = MaybeUninit::uninit();
static mut _ARRAY_TYPE_: MaybeUninit<Ref<RType>> = MaybeUninit::uninit();
static mut _MAP_TYPE_: MaybeUninit<Ref<RType>> = MaybeUninit::uninit();
static mut _FUNCTION_TYPE_: MaybeUninit<Ref<RType>> = MaybeUninit::uninit();
static mut _OPTION_TYPE_: MaybeUninit<Ref<RType>> = MaybeUninit::uninit();

static mut _MODULE_TYPE_: MaybeUninit<Ref<RType>> = MaybeUninit::uninit();

static mut _AST_TYPE_: MaybeUninit<Ref<RType>> = MaybeUninit::uninit();
static mut _SCRIPT_CODE_TYPE_: MaybeUninit<Ref<RType>> = MaybeUninit::uninit();

static mut _ARRAY_ITER_TYPE_: MaybeUninit<Ref<RType>> = MaybeUninit::uninit();
static mut _TUPLE_ITER_TYPE_: MaybeUninit<Ref<RType>> = MaybeUninit::uninit();

static mut _NULL_VALUE_: MaybeUninit<Ref<RNull>> = MaybeUninit::uninit();
static mut _TRUE_VALUE_: MaybeUninit<Ref<RBool>> = MaybeUninit::uninit();
static mut _FALSE_VALUE_: MaybeUninit<Ref<RBool>> = MaybeUninit::uninit();
static mut _NONE_VALUE_: MaybeUninit<Ref<ROption>> = MaybeUninit::uninit();

pub fn type_type() -> &'static Ref<RType> {
    unsafe { _TYPE_TYPE_.assume_init_ref() }
}
pub fn null_type() -> &'static Ref<RType> {
    unsafe { _NULL_TYPE_.assume_init_ref() }
}
pub fn bool_type() -> &'static Ref<RType> {
    unsafe { _BOOL_TYPE_.assume_init_ref() }
}
pub fn int_type() -> &'static Ref<RType> {
    unsafe { _INT_TYPE_.assume_init_ref() }
}
pub fn float_type() -> &'static Ref<RType> {
    unsafe { _FLOAT_TYPE_.assume_init_ref() }
}
pub fn string_type() -> &'static Ref<RType> {
    unsafe { _STRING_TYPE_.assume_init_ref() }
}
pub fn tuple_type() -> &'static Ref<RType> {
    unsafe { _TUPLE_TYPE_.assume_init_ref() }
}
pub fn array_type() -> &'static Ref<RType> {
    unsafe { _ARRAY_TYPE_.assume_init_ref() }
}
pub fn map_type() -> &'static Ref<RType> {
    unsafe { _MAP_TYPE_.assume_init_ref() }
}
pub fn function_type() -> &'static Ref<RType> {
    unsafe { _FUNCTION_TYPE_.assume_init_ref() }
}
pub fn option_type() -> &'static Ref<RType> {
    unsafe { _OPTION_TYPE_.assume_init_ref() }
}

pub fn module_type() -> &'static Ref<RType> {
    unsafe { _MODULE_TYPE_.assume_init_ref() }
}

pub fn ast_type() -> &'static Ref<RType> {
    unsafe { _AST_TYPE_.assume_init_ref() }
}
pub fn script_code_type() -> &'static Ref<RType> {
    unsafe { _SCRIPT_CODE_TYPE_.assume_init_ref() }
}

pub fn array_iter_type() -> &'static Ref<RType> {
    unsafe { _ARRAY_ITER_TYPE_.assume_init_ref() }
}
pub fn tuple_iter_type() -> &'static Ref<RType> {
    unsafe { _TUPLE_ITER_TYPE_.assume_init_ref() }
}

pub fn null() -> &'static Ref<RNull> {
    unsafe { _NULL_VALUE_.assume_init_ref() }
}
pub fn true_() -> &'static Ref<RBool> {
    unsafe { _TRUE_VALUE_.assume_init_ref() }
}
pub fn false_() -> &'static Ref<RBool> {
    unsafe { _FALSE_VALUE_.assume_init_ref() }
}
pub fn none() -> &'static Ref<ROption> {
    unsafe { _NONE_VALUE_.assume_init_ref() }
}

pub(crate) fn _create_type_and_string_type() -> Result<(), Error> {
    unsafe {
        let type_name = runtime().string_pool_create_without_type("Type")?;
        let string_name = runtime().string_pool_create_without_type("String")?;

        let type_tp = runtime().new_type_without_type(type_name.clone().force_into())?;
        let string_tp = runtime().new_type_without_type(string_name.clone().force_into())?;

        let tp = type_tp.clone();
        let type_tp = type_tp.init_type(tp.force_into());
        let string_tp = string_tp.init_type(type_tp.clone());

        type_name.init_type(type_tp.clone());
        string_name.init_type(type_tp.clone());

        _TYPE_TYPE_.write(type_tp.clone());
        _STRING_TYPE_.write(string_tp.clone());
    }
    Ok(())
}

pub(crate) fn _create_builtin_types() -> Result<(), Error> {
    unsafe {
        let tp = RType::new_with_str("Null")?;
        _NULL_TYPE_.write(tp.clone());

        let tp = RType::new_with_str("Bool")?;
        _BOOL_TYPE_.write(tp.clone());

        let tp = RType::new_with_str("Int")?;
        _INT_TYPE_.write(tp.clone());

        let tp = RType::new_with_str("FLoat")?;
        _FLOAT_TYPE_.write(tp.clone());

        let tp = RType::new_with_str("Tuple")?;
        _TUPLE_TYPE_.write(tp.clone());

        let tp = RType::new_with_str("Array")?;
        _ARRAY_TYPE_.write(tp.clone());

        let tp = RType::new_with_str("Map")?;
        _MAP_TYPE_.write(tp.clone());

        let tp = RType::new_with_str("Function")?;
        _FUNCTION_TYPE_.write(tp.clone());

        let tp = RType::new_with_str("Option")?;
        _OPTION_TYPE_.write(tp.clone());

        let tp = RType::new_with_str("Module")?;
        _MODULE_TYPE_.write(tp.clone());

        let tp = RType::new_with_str("Ast")?;
        _AST_TYPE_.write(tp.clone());

        let tp = RType::new_with_str("ScriptCode")?;
        _SCRIPT_CODE_TYPE_.write(tp.clone());

        let tp = RType::new_with_str("ArrayIter")?;
        _ARRAY_ITER_TYPE_.write(tp.clone());

        let tp = RType::new_with_str("TupleIter")?;
        _TUPLE_ITER_TYPE_.write(tp.clone());
    }

    Ok(())
}

pub(crate) fn _init_builtin_types() -> Result<(), Error> {
    _init_type_type(type_type().clone())?;
    _init_type_string(string_type().clone())?;

    _init_type_null(null_type().clone())?;
    _init_type_bool(bool_type().clone())?;
    _init_type_int(int_type().clone())?;
    _init_type_float(float_type().clone())?;
    _init_type_tuple(tuple_type().clone())?;
    _init_type_array(array_type().clone())?;
    _init_type_map(map_type().clone())?;
    _init_type_function(function_type().clone())?;
    _init_type_option(option_type().clone())?;

    _init_type_module(module_type().clone())?;

    _init_type_ast(ast_type().clone())?;
    _init_type_script_code(script_code_type().clone())?;

    _init_type_arrayiter(array_iter_type().clone())?;
    _init_type_tupleiter(tuple_iter_type().clone())?;

    Ok(())
}

pub(crate) fn _init_builtin_global() -> Result<(), Error> {
    set_global_with_str("null", null().cast_value())?;
    set_global_with_str("true", true_().cast_value())?;
    set_global_with_str("false", false_().cast_value())?;

    set_global_with_str("Type", type_type().cast_value())?;
    set_global_with_str("Null", null_type().cast_value())?;
    set_global_with_str("Bool", bool_type().cast_value())?;
    set_global_with_str("Int", int_type().cast_value())?;
    set_global_with_str("Float", float_type().cast_value())?;
    set_global_with_str("String", string_type().cast_value())?;
    set_global_with_str("Tuple", tuple_type().cast_value())?;
    set_global_with_str("Array", array_type().cast_value())?;
    set_global_with_str("Map", map_type().cast_value())?;
    set_global_with_str("Function", function_type().cast_value())?;
    set_global_with_str("Option", option_type().cast_value())?;
    set_global_with_str("ScriptCode", script_code_type().cast_value())?;
    set_global_with_str("Module", module_type().cast_value())?;

    Ok(())
}

pub(crate) fn _init_builtin_values() -> Result<(), Error> {
    unsafe {
        _NULL_VALUE_.write(_new_null_value()?);
        _TRUE_VALUE_.write(_new_bool_value(true)?);
        _FALSE_VALUE_.write(_new_bool_value(false)?);
        _NONE_VALUE_.write(ROption::new_(None)?);
    }
    Ok(())
}
