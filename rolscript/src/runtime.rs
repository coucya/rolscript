use core::mem::size_of;
use core::mem::MaybeUninit;
use core::ptr::NonNull;

use crate::alloc::Allocator;
use crate::collections::*;

use crate::error::*;

use crate::ast::ast_as_code;
use crate::lexical::Lexical;
use crate::parser::Parser;

use crate::array::*;
use crate::function::*;
use crate::module::*;
use crate::number::*;
use crate::script_code::*;
use crate::string::*;
use crate::type_::*;
use crate::value::*;

use crate::builtin::*;

use crate::nonnull_of;
use crate::util::StringMap;

pub trait Loader {
    fn normalize_name(
        &mut self,
        requester: Ref<RModule>,
        name: Ref<RString>,
    ) -> Result<Ref<RString>, Error>;

    fn load(&mut self, normalized_name: Ref<RString>) -> Result<Ref<RFunction>, Error>;
}

pub trait Visitor {
    fn visit(&mut self, value: NonNull<GcHeader>);

    fn visit_value(&mut self, value: &RValue) {
        self.visit(value.as_nonnull_ptr())
    }

    fn visit_ptr(&mut self, value_ptr: NonNull<GcHeader>) {
        self.visit(value_ptr)
    }
}

impl<T> Visitor for T
where
    T: FnMut(NonNull<GcHeader>),
{
    fn visit(&mut self, value: NonNull<GcHeader>) {
        self(value);
    }
}

pub const SMALL_INTEGER_BEG: isize = -256;
pub const SMALL_INTEGER_END: isize = 512;

pub(crate) static mut _RUNTIME_: MaybeUninit<Runtime> = MaybeUninit::uninit();
pub(crate) static mut _RUNTIME_IS_INITIALIZED_: bool = false;

#[inline]
pub fn runtime() -> &'static mut Runtime {
    unsafe { _RUNTIME_.assume_init_mut() }
}

#[inline]
pub fn is_initialized() -> bool {
    unsafe { _RUNTIME_IS_INITIALIZED_ }
}

pub fn initialize(
    allocator: &'static dyn Allocator,
    loader: &'static mut dyn Loader,
) -> Result<(), Error> {
    unsafe {
        if !_RUNTIME_IS_INITIALIZED_ {
            _initialize(allocator, loader)?;
        }
        Ok(())
    }
}

pub fn finalize() {
    unsafe {
        if _RUNTIME_IS_INITIALIZED_ {
            _RUNTIME_.assume_init_drop();
            _RUNTIME_IS_INITIALIZED_ = false;
        }
    }
}

fn _initialize(
    allocator: &'static dyn Allocator,
    loader: &'static mut dyn Loader,
) -> Result<(), Error> {
    unsafe {
        let runtime = Runtime::new(allocator, loader)?;
        _RUNTIME_.write(runtime);
    }

    unsafe {
        _create_type_and_string_type()?;
        _create_builtin_types()?;

        _init_builtin_types()?;

        _init_builtin_values()?;

        _init_integer_pool()?;

        _init_builtin_global()?;

        _RUNTIME_IS_INITIALIZED_ = true;
    }

    Ok(())
}

pub(crate) fn _init_integer_pool() -> Result<(), Error> {
    let runtime = runtime();
    runtime
        ._small_integer_pool
        .reserve((SMALL_INTEGER_END - SMALL_INTEGER_BEG).max(0) as usize)
        .map_err(|_| Error::new_outofmemory())?;

    for n in SMALL_INTEGER_BEG..SMALL_INTEGER_END {
        let v = _new_int_value(n)?;
        runtime
            ._small_integer_pool
            .push(v)
            .map_err(|_| Error::new_outofmemory())?;
    }
    Ok(())
}

struct Frame {
    _callee: RValue,
}

#[allow(dead_code)]
impl Frame {
    pub(self) fn new(callee: RValue) -> Self {
        Self { _callee: callee }
    }

    pub fn get_callee(&self) -> RValue {
        self._callee.clone()
    }
}

struct GcInfo {
    pub(self) _current_obj_count: usize,
    pub(self) _curent_mem_size: usize,
    pub(self) _last_obj_count: usize,
    pub(self) _last_mem_size: usize,
    pub(self) _gc_objs: ListBase,
    pub(self) _tmp_gc_objs: ListBase,
    pub(self) _to_be_released_objs: ListBase,
    pub(self) _has_been_released_objs: ListBase,
}

impl GcInfo {
    pub(self) unsafe fn init(mut ptr: NonNull<Self>) {
        use core::ptr::addr_of_mut;
        addr_of_mut!(ptr.as_mut()._current_obj_count).write(0);
        addr_of_mut!(ptr.as_mut()._curent_mem_size).write(0);
        addr_of_mut!(ptr.as_mut()._last_obj_count).write(0);
        addr_of_mut!(ptr.as_mut()._last_mem_size).write(0);
        ListBase::init(nonnull_of!(ptr.as_mut()._gc_objs));
        ListBase::init(nonnull_of!(ptr.as_mut()._tmp_gc_objs));
        ListBase::init(nonnull_of!(ptr.as_mut()._to_be_released_objs));
        ListBase::init(nonnull_of!(ptr.as_mut()._has_been_released_objs));
    }
}

pub struct Runtime {
    _allocator: &'static dyn Allocator,
    _frames: Array<Frame>,

    _gc_info: NonNull<GcInfo>,

    _string_pool: StringPool,
    _small_integer_pool: Array<Ref<RInt>>,

    _loader: &'static mut dyn Loader,
    _modules: StringMap<Ref<RModule>>,

    _global: StringMap<RValue>,
}

type CResult<T> = Result<T, Error>;

impl Runtime {
    pub(crate) fn new(
        allocator: &'static dyn Allocator,
        loader: &'static mut dyn Loader,
    ) -> CResult<Self> {
        let gc_info = unsafe {
            let gc_info_ptr = allocator.alloc_block(size_of::<GcInfo>()).cast::<GcInfo>();

            if gc_info_ptr.is_null() {
                return Err(Error::new_outofmemory());
            }

            let ptr = NonNull::new_unchecked(gc_info_ptr);
            GcInfo::init(ptr);

            ptr
        };

        let runtime = Self {
            _allocator: allocator,

            _gc_info: gc_info,

            _string_pool: StringPool::new(allocator),
            _small_integer_pool: Array::new(allocator),
            _frames: Array::new(allocator),

            _loader: loader,
            _modules: StringMap::new(allocator),

            _global: StringMap::new(allocator),
        };

        Ok(runtime)
    }

    #[inline]
    pub(crate) fn _call(
        &mut self,
        callee: &RValue,
        this_value: &RValue,
        args: &[RValue],
    ) -> CResult<RValue> {
        let frame = Frame::new(callee.clone());
        if self._frames.push(frame).is_err() {
            return Err(Error::OutOfMemory);
        }

        let ret = _value_call_raw(callee, this_value, args);

        self._frames.pop();

        ret
    }
}

// 对象分配方法与gc
impl Runtime {
    fn gc_info_mut(&mut self) -> &mut GcInfo {
        unsafe { self._gc_info.as_mut() }
    }

    fn _dec_ref(&mut self) {
        fn _dec_ref_mark(mut value: NonNull<GcHeader>) {
            unsafe {
                let value_ref = value.as_mut();
                debug_assert!(value_ref.ref_count() > 0);

                value_ref.dec_ref();

                if value_ref.mark() && value_ref.ref_count() == 0 {
                    runtime().gc_info_mut()._gc_objs.remove(value);
                    runtime().gc_info_mut()._tmp_gc_objs.insert_last(value);
                }
            }
        }

        unsafe {
            for node in self.gc_info_mut()._gc_objs.iter() {
                let mut value = node.cast::<GcHeader>();

                debug_assert!(!value.as_mut().mark());

                value_visit_ptr(&mut _dec_ref_mark, value);
                value.as_mut().set_mark(true);

                if value.as_ref().ref_count() == 0 {
                    self.gc_info_mut()._gc_objs.remove(value);
                    self.gc_info_mut()._tmp_gc_objs.insert_last(value);
                }
            }
        }
    }
    fn _scan(&mut self) {
        fn _scan_1_mark(mut value: NonNull<GcHeader>) {
            unsafe {
                let value_ref = value.as_mut();

                value_ref.inc_ref();

                if value_ref.ref_count() == 1 {
                    runtime().gc_info_mut()._tmp_gc_objs.remove(value);
                    runtime().gc_info_mut()._gc_objs.insert_last(value);
                    value_ref.set_mark(false);
                }
            }
        }

        fn _scan_2_mark(mut value: NonNull<GcHeader>) {
            unsafe {
                let value_ref = value.as_mut();
                value_ref.inc_ref();
            }
        }

        unsafe {
            for node in self.gc_info_mut()._gc_objs.iter() {
                let mut value = node.cast::<GcHeader>();

                debug_assert!(value.as_mut().ref_count() > 0);

                value.as_mut().set_mark(false);

                value_visit_ptr(&mut _scan_1_mark, value);
            }

            for node in self.gc_info_mut()._tmp_gc_objs.iter() {
                let value = node.cast::<GcHeader>();
                value_visit_ptr(&mut _scan_2_mark, value);
            }
        }
    }

    fn _free_cycles(&mut self) -> Result<(), Error> {
        for node in self.gc_info_mut()._tmp_gc_objs.iter() {
            let value = node.cast::<GcHeader>();
            self.gc_info_mut()._tmp_gc_objs.remove(value);
            self.gc_info_mut()._to_be_released_objs.insert_last(value);
        }

        for node in self.gc_info_mut()._to_be_released_objs.iter() {
            let value = node.cast::<GcHeader>();
            self.gc_info_mut()._to_be_released_objs.remove(value);
            self.gc_info_mut()
                ._has_been_released_objs
                .insert_last(value);

            unsafe {
                let v = Ref::from_raw(value);
                value_destory(&v)?;
            }
        }

        for node in self.gc_info_mut()._has_been_released_objs.iter() {
            let value = node.cast::<GcHeader>();
            self.gc_info_mut()._has_been_released_objs.remove(value);

            self.free_gc_obj(value);
        }

        Ok(())
    }

    fn run_gc(&mut self) -> Result<(), Error> {
        #[cfg(debug_assertions)]
        {
            dbg!(self.gc_info_mut()._curent_mem_size);
            dbg!(self.gc_info_mut()._current_obj_count);
            dbg!(self.gc_info_mut()._last_mem_size);
            dbg!(self.gc_info_mut()._last_obj_count);
        }

        let last_mem_size = self.gc_info_mut()._curent_mem_size;
        let last_obj_ount = self.gc_info_mut()._current_obj_count;

        self._dec_ref();
        self._scan();
        self._free_cycles()?;

        self.gc_info_mut()._last_mem_size = last_mem_size;
        self.gc_info_mut()._last_obj_count = last_obj_ount;

        Ok(())
    }

    fn free_gc_obj(&mut self, value: NonNull<GcHeader>) {
        unsafe {
            let align = size_of::<usize>();
            let size = value.as_ref().block_size();

            GcHeader::drop_head(value);

            self._allocator.free(value.as_ptr() as _, size, align);

            self.gc_info_mut()._curent_mem_size -= size;
            self.gc_info_mut()._current_obj_count -= 1;
        }
    }

    fn new_gc_obj(&mut self, size: usize, type_: Ref<RType>) -> CResult<Ref<GcHeader>> {
        let size = size.max(size_of::<GcHeader>());

        {
            if self.gc_info_mut()._last_mem_size == 0 {
                if self.gc_info_mut()._curent_mem_size > 1024 * 1024 {
                    self.run_gc()?;
                }
            } else if self.gc_info_mut()._curent_mem_size > self.gc_info_mut()._last_mem_size * 8 {
                self.run_gc()?;
            }
        }

        unsafe {
            let align = size_of::<usize>();
            let ptr = self._allocator.alloc(size, align) as *mut GcHeader;
            if ptr.is_null() {
                return Err(Error::OutOfMemory);
            }

            let ptr = NonNull::new_unchecked(ptr);
            GcHeader::init(ptr, size, Some(type_));

            self.gc_info_mut()._gc_objs.insert_last(ptr);
            self.gc_info_mut()._current_obj_count += 1;
            self.gc_info_mut()._curent_mem_size += size;

            let v = Ref::from_raw(ptr);

            Ok(v)
        }
    }

    pub(crate) fn new_gc_obj_without_type(
        &mut self,
        size: usize,
    ) -> CResult<TypeUninitRef<GcHeader>> {
        let size = size.max(size_of::<GcHeader>());

        unsafe {
            let align = size_of::<usize>();
            let ptr = self._allocator.alloc(size, align) as *mut GcHeader;
            if ptr.is_null() {
                return Err(Error::OutOfMemory);
            }

            let ptr = NonNull::new_unchecked(ptr);
            GcHeader::init(ptr, size, None);

            self.gc_info_mut()._gc_objs.insert_last(ptr);
            self.gc_info_mut()._current_obj_count += 1;
            self.gc_info_mut()._curent_mem_size += size;

            let v = TypeUninitRef::from_raw(ptr);

            Ok(v)
        }
    }

    pub(crate) fn string_pool_get(&mut self, string: &str) -> Result<Ref<RString>, Error> {
        unsafe {
            if let Some(s) = self._string_pool.get(string) {
                Ok(s)
            } else {
                let new_string = self
                    .new_gc_obj(RString::need_size(string), string_type().clone())?
                    .cast::<RString>();

                RString::init(new_string.as_nonnull_ptr(), string);

                self._string_pool.add(new_string.clone())?;

                Ok(new_string)
            }
        }
    }

    pub(crate) fn string_pool_create_without_type(
        &mut self,
        string: &str,
    ) -> Result<TypeUninitRef<RString>, Error> {
        unsafe {
            let new_string = self
                .new_gc_obj_without_type(RString::need_size(string))?
                .cast::<RString>();

            RString::init(new_string.as_nonnull_ptr(), string);

            self._string_pool.add(new_string.clone().force_into())?;

            Ok(new_string)
        }
    }

    pub(crate) fn new_type_without_type(
        &mut self,
        name: Ref<RString>,
    ) -> Result<TypeUninitRef<RType>, Error> {
        unsafe {
            let tp_uninit: TypeUninitRef<RType> =
                self.new_gc_obj_without_type(RType::need_size())?.cast();

            RType::init(self._allocator, tp_uninit.as_nonnull_ptr(), name);

            Ok(tp_uninit)
        }
    }

    pub(crate) fn integer_pool_get(&mut self, n: Int) -> Result<Ref<RInt>, Error> {
        if n >= SMALL_INTEGER_BEG && n < SMALL_INTEGER_END {
            unsafe {
                let slice = self._small_integer_pool.as_slice();
                Ok(slice
                    .get_unchecked((n - SMALL_INTEGER_BEG) as usize)
                    .clone())
            }
        } else {
            _new_int_value(n)
        }
    }
}

pub fn allocator() -> &'static dyn Allocator {
    runtime()._allocator
}

pub fn loader() -> &'static mut dyn Loader {
    runtime()._loader
}

pub fn run_gc() -> Result<(), Error> {
    runtime().run_gc()
}

pub fn new_gc_obj(size: usize, type_: Ref<RType>) -> Result<Ref<GcHeader>, Error> {
    runtime().new_gc_obj(size, type_)
}

pub fn get_global(name: &Ref<RString>) -> Option<RValue> {
    runtime()._global.get(name).cloned()
}

pub fn set_global(name: Ref<RString>, value: RValue) -> Result<(), Error> {
    runtime()._global.insert(name, value)?;
    Ok(())
}

pub fn set_global_with_str(name: &str, value: RValue) -> Result<(), Error> {
    let key = RString::new(name)?;
    runtime()._global.insert(key, value)?;
    Ok(())
}

pub fn load_module(requester: &Ref<RModule>, name: &Ref<RString>) -> Result<Ref<RModule>, Error> {
    let normalized = loader().normalize_name(requester.clone(), name.clone())?;
    if let Some(module) = runtime()._modules.get(&normalized) {
        Ok(module.clone())
    } else {
        let func = loader().load(normalized.clone())?;
        let module = RModule::new(normalized.clone(), Some(func.clone()))?;

        runtime()._modules.insert(normalized, module.clone())?;

        value_call_with_this(func.cast_value_ref(), module.cast_value_ref(), &[])?;

        Ok(module)
    }
}

pub fn parse_to_code(script_code: &str, allow_last_expr: bool) -> Result<Ref<RScriptCode>, Error> {
    let ast_node = {
        let lexical = Lexical::new(script_code);
        let mut parser = Parser::new(lexical);
        parser.parse(allow_last_expr)?
    };
    ast_as_code(ast_node, false)
}

pub fn parse_to_function(script_code: &str) -> Result<Ref<RFunction>, Error> {
    let code = parse_to_code(script_code, true)?;
    let caps = RArray::new()?;
    let func = RFunction::from_script_code(code, caps)?;
    Ok(func)
}

pub fn eval(script_code: &str) -> Result<RValue, Error> {
    let func = parse_to_function(script_code)?;
    let null_v = null().cast_value();
    runtime()._call(&func.cast_value(), &null_v, &[])
}

pub fn eval_with_module(module: &Ref<RModule>, script_code: &str) -> Result<RValue, Error> {
    let func = parse_to_function(script_code)?;
    let module = module.cast_value();
    runtime()._call(&func.cast_value(), &module, &[])
}
