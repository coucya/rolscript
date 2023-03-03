#![allow(non_snake_case)]

use core::mem::size_of;
use core::ptr::addr_of_mut;
use core::ptr::NonNull;

use crate::runtime::*;

use crate::error::*;

use crate::number::*;
use crate::string::*;
use crate::type_::*;
use crate::value::*;

use crate::builtin::*;

use crate::util::expect_arg1;
use crate::util::ValueMap;

#[repr(C)]
pub struct RMap {
    _headerr: GcHeader,
    _map: ValueMap<RValue>,
}

impl RMap {
    unsafe fn init(mut ptr: NonNull<Self>) {
        addr_of_mut!(ptr.as_mut()._map).write(ValueMap::new(allocator()));
    }

    unsafe fn _drop(&mut self) {
        addr_of_mut!(self._map).drop_in_place();
    }

    pub fn new() -> Result<Ref<Self>, Error> {
        let tp = map_type().clone();
        unsafe {
            let v = new_gc_obj(size_of::<Self>(), tp)?.cast::<Self>();
            Self::init(v.as_nonnull_ptr());
            Ok(v)
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self._map.len()
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self._map.capacity()
    }

    #[inline]
    pub fn contains_key(&self, key: &RValue) -> bool {
        self._map.contains_key(key)
    }

    #[inline]
    pub fn set(&mut self, key: RValue, value: RValue) -> Result<Option<RValue>, Error> {
        self._map.insert(key, value).map_err(|_| Error::OutOfMemory)
    }

    #[inline]
    pub fn get(&self, key: &RValue) -> Option<&RValue> {
        self._map.get(key)
    }

    #[inline]
    pub fn get_key_value(&self, key: &RValue) -> Option<(&RValue, &RValue)> {
        self._map.get_key_value(&key)
    }

    #[inline]
    pub fn remove(&mut self, key: &RValue) -> Option<RValue> {
        self._map.remove(key)
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = (&RValue, &RValue)> {
        self._map.iter()
    }
}

pub(crate) fn _init_type_map(mut tp: Ref<RType>) -> Result<(), Error> {
    tp.with_visit(map__visit);

    tp.with_destory(map__destory);

    tp.with_get_attr(map__get_attr);
    tp.with_set_attr(map__set_attr);

    tp.with_get_item(map__get_item);
    tp.with_set_item(map__set_item);

    tp.with_eq(default_value_eq);
    tp.with_hash(default_value_hash);
    tp.with_str(default_value_str);

    tp.add_method_str_light("len", map__len)?;
    tp.add_method_str_light("contains_key", map__contains_key)?;

    Ok(())
}

use crate::runtime::Visitor;

fn map__visit(visitor: &mut dyn Visitor, value_ptr: NonNull<GcHeader>) {
    unsafe {
        let arr = value_ptr.cast::<RMap>();
        for (k, v) in arr.as_ref().iter() {
            visitor.visit_value(k);
            visitor.visit_value(v);
        }
    }
}

fn map__destory(value: &RValue) -> Result<(), Error> {
    unsafe {
        let mut m = value.expect_cast::<RMap>(map_type())?;
        m._drop();
        Ok(())
    }
}

fn map__get_attr(value: &RValue, name: &Ref<RString>) -> Result<RValue, Error> {
    let map = unsafe { value.expect_cast::<RMap>(map_type())? };

    if let Some(v) = map.get(unsafe { name.cast_ref() }) {
        Ok(v.clone())
    } else {
        Ok(null().cast_value())
    }
}

fn map__set_attr(value: &RValue, name: &Ref<RString>, attr_value: &RValue) -> Result<(), Error> {
    let mut map = unsafe { value.expect_cast::<RMap>(map_type())? };
    map.set(name.cast_value(), attr_value.clone())?;
    Ok(())
}

fn map__get_item(value: &RValue, index: &RValue) -> Result<RValue, Error> {
    let map = unsafe { value.expect_cast::<RMap>(map_type())? };

    if let Some(v) = map.get(&index) {
        Ok(v.clone())
    } else {
        Ok(null().cast_value())
    }
}

fn map__set_item(value: &RValue, index: &RValue, item: &RValue) -> Result<(), Error> {
    let mut map = unsafe { value.expect_cast::<RMap>(map_type())? };
    map.set(index.clone(), item.clone())?;
    Ok(())
}

fn map__len(this: &RValue, _args: &[RValue]) -> Result<RValue, Error> {
    let map = unsafe { this.expect_cast::<RMap>(map_type())? };
    Ok(RInt::new(map.len() as Int)?.cast_value())
}

fn map__contains_key(this: &RValue, args: &[RValue]) -> Result<RValue, Error> {
    let map = unsafe { this.expect_cast::<RMap>(map_type())? };
    let key = expect_arg1(args)?;
    let b = map.contains_key(&key);
    Ok(RBool::new(b)?.cast_value())
}
