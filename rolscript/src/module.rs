use core::mem::size_of;
use core::ptr::addr_of_mut;
use core::ptr::NonNull;

use crate::runtime::*;

use crate::error::*;
use crate::runtime_error_fmt;

use crate::util::StringMap;

use crate::function::RFunction;
use crate::string::*;
use crate::type_::*;
use crate::value::*;

use crate::builtin::*;

#[repr(C)]
pub struct RModule {
    _header: GcHeader,
    _normalized_name: Ref<RString>,
    _init_func: Option<Ref<RFunction>>,
    _attrs: StringMap<RValue>,
}

impl RModule {
    unsafe fn init(
        mut ptr: NonNull<Self>,
        normalize_name: Ref<RString>,
        init_func: Option<Ref<RFunction>>,
    ) {
        addr_of_mut!(ptr.as_mut()._normalized_name).write(normalize_name);
        addr_of_mut!(ptr.as_mut()._init_func).write(init_func);
        addr_of_mut!(ptr.as_mut()._attrs).write(StringMap::new(allocator()));
    }

    unsafe fn _drop(&mut self) {
        addr_of_mut!(self._normalized_name).drop_in_place();
        addr_of_mut!(self._init_func).drop_in_place();
        addr_of_mut!(self._attrs).drop_in_place();
    }

    pub fn new(
        normalize_name: Ref<RString>,
        init_func: Option<Ref<RFunction>>,
    ) -> Result<Ref<Self>, Error> {
        let tp = module_type().clone();
        unsafe {
            let v = new_gc_obj(size_of::<Self>(), tp)?.cast::<Self>();
            Self::init(v.as_nonnull_ptr(), normalize_name, init_func);
            Ok(v)
        }
    }

    pub fn normalized_name(&self) -> &Ref<RString> {
        &self._normalized_name
    }
}

pub(crate) fn _init_type_module(mut tp: Ref<RType>) -> Result<(), Error> {
    tp.with_destory(module__destory);

    tp.with_get_attr(module__get_attr);
    tp.with_set_attr(module__set_attr);

    tp.with_eq(default_value_eq);
    tp.with_hash(default_value_hash);
    tp.with_str(default_value_str);

    tp.add_method_str_light("import", module__import)?;

    Ok(())
}

#[allow(non_snake_case)]
fn module__destory(value: &RValue) -> Result<(), Error> {
    unsafe {
        let mut m = value.expect_cast::<RModule>(module_type())?;
        m._drop();
        Ok(())
    }
}

#[allow(non_snake_case)]
fn module__get_attr(instance: &RValue, name: &Ref<RString>) -> Result<RValue, Error> {
    let m = unsafe { instance.expect_cast::<RModule>(module_type())? };

    if let Some(v) = m._attrs.get(name) {
        return Ok(v.clone());
    } else {
        return Err(runtime_error_fmt!(
            "module \"{}\" has no attribute \"{}\"",
            m._normalized_name.as_str(),
            name.as_str()
        ));
    }
}

#[allow(non_snake_case)]
fn module__set_attr(
    instance: &RValue,
    name: &Ref<RString>,
    attr_value: &RValue,
) -> Result<(), Error> {
    let mut m = unsafe { instance.expect_cast::<RModule>(module_type())? };
    m._attrs.insert(name.clone(), attr_value.clone())?;
    Ok(())
}

#[allow(non_snake_case)]
fn module__import(this: &RValue, args: &[RValue]) -> Result<RValue, Error> {
    use crate::util::expect_arg1;
    let import_name = expect_arg1(args)?;

    let this_module = unsafe { this.expect_cast::<RModule>(module_type())? };
    let import_name = unsafe { import_name.expect_cast::<RString>(string_type())? };

    let module = load_module(&this_module, &import_name)?;

    Ok(module.cast_value())
}
