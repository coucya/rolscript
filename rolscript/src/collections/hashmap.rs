use core::hash::Hash;
use core::hash::Hasher;
use core::mem::size_of;
use core::mem::MaybeUninit;
use core::ptr::NonNull;
use core::ptr::{addr_of, addr_of_mut};

use crate::alloc::Allocator;

use super::list::*;

use crate::nonnull_of;

struct MyHasher {
    _hash: u64,
    _seed: u64,
}

impl MyHasher {
    const SEED_LIST: &[u64] = &[31, 131, 1313, 13131];

    fn new_seed() -> u64 {
        let tmp = 0;
        let seed_idx = core::ptr::addr_of!(tmp) as usize;
        let seed = Self::SEED_LIST[(seed_idx / size_of::<usize>()) % Self::SEED_LIST.len()];
        seed
    }

    fn new(seed: u64) -> Self {
        Self {
            _hash: 0,
            _seed: seed,
        }
    }
}

impl Hasher for MyHasher {
    fn finish(&self) -> u64 {
        self._hash
    }

    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            // hash = hash * seed + byte
            self._hash = self
                ._hash
                .wrapping_mul(self._seed)
                .wrapping_add(*byte as u64);
        }
    }
}

type Bucket<K, V> = List<MaybeUninit<(u64, K, V)>>;
type Node<K, V> = ListNode<MaybeUninit<(u64, K, V)>>;

#[cfg(debug_assertions)]
#[derive(Clone, Copy, Debug)]
struct Debuginfo {
    pub lockup_count: usize,
    pub lockup_total: usize,
    pub lockup_max: usize,
    pub lockup_min: usize,
    pub lockup_avg: f64,
}
#[cfg(debug_assertions)]
impl Debuginfo {
    fn new() -> Self {
        Self {
            lockup_count: 0,
            lockup_total: 0,
            lockup_max: 0,
            lockup_min: usize::MAX,
            lockup_avg: 0.0,
        }
    }

    fn update_lockup(&mut self, n: usize) {
        self.lockup_count += 1;
        self.lockup_total += n;
        if n > self.lockup_max {
            self.lockup_max = n;
        }
        if n < self.lockup_min {
            self.lockup_min = n;
        }

        self.lockup_avg = self.lockup_total as f64 / self.lockup_count as f64;
    }

    fn merge(&mut self, other: &Self) {
        self.lockup_count += other.lockup_count;
        self.lockup_total += other.lockup_total;
        if other.lockup_max > self.lockup_max {
            self.lockup_max = other.lockup_max;
        }
        if other.lockup_min < self.lockup_min {
            self.lockup_min = other.lockup_min;
        }
        self.lockup_avg = self.lockup_total as f64 / self.lockup_count as f64;
    }
}

#[repr(C)]
pub struct FixedMap<K, V> {
    #[cfg(debug_assertions)]
    _debug: Debuginfo,
    _hash_seed: u64,
    _len: usize,
    _capacity: usize,
    _free: ListBase,
    _data: [(Bucket<K, V>, Node<K, V>); 1],
}

impl<K, V> FixedMap<K, V> {
    #[cfg(debug_assertions)]
    fn debug_info(&self) -> Debuginfo {
        self._debug
    }

    fn need_size(capacity: usize) -> usize {
        size_of::<Self>() + size_of::<(Bucket<K, V>, Node<K, V>)>() * capacity
    }

    unsafe fn init(mut ptr: NonNull<Self>, capacity: usize) {
        unsafe {
            let seed = MyHasher::new_seed();
            let r = ptr.as_mut();

            #[cfg(debug_assertions)]
            {
                addr_of_mut!(r._debug).write(Debuginfo::new());
            }

            addr_of_mut!(r._hash_seed).write(seed);
            addr_of_mut!(r._len).write(0);
            addr_of_mut!(r._capacity).write(capacity);
            ListBase::init(nonnull_of!(r._free));

            for i in 0..capacity {
                Bucket::init(r._get_bucket_ptr(i));
                Node::init(r._get_node_ptr(i), MaybeUninit::uninit());
            }

            for i in 0..capacity {
                r._free.insert_last(r._get_node_ptr(i));
            }
        }
    }

    unsafe fn _get_bucket_ptr(&self, i: usize) -> NonNull<Bucket<K, V>> {
        nonnull_of!(self._data.get_unchecked(i).0)
    }
    unsafe fn _get_node_ptr(&self, i: usize) -> NonNull<Node<K, V>> {
        nonnull_of!(self._data.get_unchecked(i).1)
    }

    pub unsafe fn free(allocator: &dyn Allocator, mut ptr: NonNull<Self>) {
        ptr.as_mut().clear();
        let size = Self::need_size(ptr.as_ref()._capacity);
        let align = size_of::<usize>();
        allocator.free(ptr.as_ptr() as *mut u8, size, align);
    }

    pub fn new(allocator: &dyn Allocator, capacity: usize) -> Option<NonNull<Self>> {
        if capacity < 1 {
            return None;
        }
        unsafe {
            let size = Self::need_size(capacity);
            let align = size_of::<usize>();
            let ptr = allocator.alloc(size, align) as *mut Self;
            if ptr.is_null() {
                return None;
            }
            Self::init(NonNull::new_unchecked(ptr), capacity);
            Some(NonNull::new_unchecked(ptr))
        }
    }

    fn clear(&mut self) {
        unsafe {
            for i in 0..self._capacity {
                let mut bucket_ptr = self._get_bucket_ptr(i);
                let bucket_ref = bucket_ptr.as_mut();
                while let Some(mut node) = bucket_ref.pop_front() {
                    let node_ref = node.as_mut();
                    node_ref.data_mut().assume_init_drop();
                    self._free.insert_last(node);
                }
            }
        }
    }

    fn move_to(&mut self, other: &mut Self) -> Result<(), ()>
    where
        K: Hash + Eq,
    {
        if self._len > (other._capacity - other._len) {
            return Err(());
        }

        #[cfg(debug_assertions)]
        {
            other._debug.merge(&self._debug);
        }

        #[allow(unused_must_use)]
        unsafe {
            for i in 0..self._capacity {
                let mut bucket_ptr = self._get_bucket_ptr(i);
                let bucket_ref = bucket_ptr.as_mut();
                while let Some(mut node) = bucket_ref.pop_front() {
                    let node_ref = node.as_mut();
                    let (hc, k, v) = node_ref.data().assume_init_read();
                    self._free.insert_last(node);
                    other.insert(k, v);
                }
            }
        }

        self.clear();
        Ok(())
    }

    fn move_to_when<HF, EF>(&mut self, other: &mut Self, hf: &mut HF, ef: &mut EF) -> Result<(), ()>
    where
        HF: FnMut(&K) -> Result<usize, ()>,
        EF: FnMut(&K, &K) -> Result<bool, ()>,
    {
        if self._len > (other._capacity - other._len) {
            return Err(());
        }

        #[cfg(debug_assertions)]
        {
            other._debug.merge(&self._debug);
        }

        #[allow(unused_must_use)]
        unsafe {
            for i in 0..self._capacity {
                let mut bucket_ptr = self._get_bucket_ptr(i);
                let bucket_ref = bucket_ptr.as_mut();
                while let Some(mut node) = bucket_ref.pop_front() {
                    let node_ref = node.as_mut();
                    let (hc, k, v) = node_ref.data().assume_init_read();
                    self._free.insert_last(node);
                    other.insert_when(k, v, hf, ef);
                }
            }
        }

        self.clear();
        Ok(())
    }

    fn capacity(&self) -> usize {
        self._capacity
    }
    fn len(&self) -> usize {
        self._len
    }

    #[inline]
    fn _new_hasher(&self) -> MyHasher {
        MyHasher::new(self._hash_seed)
    }

    fn _compute_hash<Q>(&self, k: &Q) -> u64
    where
        K: core::borrow::Borrow<Q>,
        Q: Hash + ?Sized,
    {
        let mut hasher = self._new_hasher();
        k.hash(&mut hasher);
        let hc = hasher.finish();
        hc
    }

    #[inline]
    fn _find_bucket_ptr(&self, hashcode: u64) -> NonNull<Bucket<K, V>> {
        unsafe {
            let idx = hashcode % self._capacity as u64;
            self._get_bucket_ptr(idx as usize)
        }
    }

    // -> (node?, hashcode, bucket_ptr)
    fn _find_node<Q>(&self, k: &Q) -> (Option<NonNull<Node<K, V>>>, u64, NonNull<Bucket<K, V>>)
    where
        K: core::borrow::Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let hashcode = self._compute_hash(k);

        #[cfg(debug_assertions)]
        let mut lc = 0;

        let bucket_ptr = self._find_bucket_ptr(hashcode);
        let bucket_ref = unsafe { bucket_ptr.as_ref() };

        let mut res = (None, hashcode, bucket_ptr);

        for node in bucket_ref.iter() {
            #[cfg(debug_assertions)]
            {
                lc += 1;
            }

            let d = unsafe { node.as_ref().data().assume_init_ref() };
            if d.0 == hashcode && d.1.borrow().eq(k) {
                res = (Some(node), hashcode, bucket_ptr);
                break;
            }
        }

        #[cfg(debug_assertions)]
        unsafe {
            if lc != 0 {
                (&mut *(self as *const Self as *mut Self))
                    ._debug
                    .update_lockup(lc);
            }
        }

        res
    }

    // -> (node?, hashcode, bucket_ptr)
    fn _find_node_when<Q, HF, EF>(
        &self,
        k: &Q,
        hf: &mut HF,
        ef: &mut EF,
    ) -> Result<(Option<NonNull<Node<K, V>>>, u64, NonNull<Bucket<K, V>>), ()>
    where
        HF: FnMut(&Q) -> Result<usize, ()>,
        EF: FnMut(&K, &Q) -> Result<bool, ()>,
    {
        let hashcode = hf(k)? as u64;

        #[cfg(debug_assertions)]
        let mut lc = 0;

        let bucket_ptr = self._find_bucket_ptr(hashcode);
        let bucket_ref = unsafe { bucket_ptr.as_ref() };

        let mut res = (None, hashcode, bucket_ptr);

        for node in bucket_ref.iter() {
            #[cfg(debug_assertions)]
            {
                lc += 1;
            }

            let d = unsafe { node.as_ref().data().assume_init_ref() };
            if d.0 == hashcode && ef(&d.1, k)? {
                res = (Some(node), hashcode, bucket_ptr);
                break;
            }
        }

        #[cfg(debug_assertions)]
        unsafe {
            if lc != 0 {
                (&mut *(self as *const Self as *mut Self))
                    ._debug
                    .update_lockup(lc);
            }
        }

        Ok(res)
    }

    pub fn insert(&mut self, k: K, v: V) -> Result<Option<V>, (K, V)>
    where
        K: Hash + Eq,
    {
        let (node, hashcode, mut bucket_ptr) = self._find_node(&k);
        let bucket_ref = unsafe { bucket_ptr.as_mut() };

        if let Some(mut node) = node {
            unsafe {
                let node = node.as_mut();
                let old = node.data().assume_init_read();
                node.data_mut().write((hashcode, k, v));
                Ok(Some(old.2))
            }
        } else {
            if let Some(node_ptr) = self._free.pop_front() {
                unsafe {
                    let mut node_ptr = as_node::<MaybeUninit<(u64, K, V)>>(node_ptr);
                    let node = node_ptr.as_mut();
                    node.data_mut().write((hashcode, k, v));
                    bucket_ref.insert_first(node_ptr);
                    self._len += 1;
                    Ok(None)
                }
            } else {
                Err((k, v))
            }
        }
    }

    pub fn insert_when<HF, EF>(
        &mut self,
        k: K,
        v: V,
        hf: &mut HF,
        ef: &mut EF,
    ) -> Result<Option<V>, (K, V)>
    where
        HF: FnMut(&K) -> Result<usize, ()>,
        EF: FnMut(&K, &K) -> Result<bool, ()>,
    {
        let (node, hashcode, mut bucket_ptr) = match self._find_node_when(&k, hf, ef) {
            Ok(v) => v,
            Err(()) => return Err((k, v)),
        };
        let bucket_ref = unsafe { bucket_ptr.as_mut() };

        if let Some(mut node) = node {
            unsafe {
                let node = node.as_mut();
                let old = node.data().assume_init_read();
                node.data_mut().write((hashcode, k, v));
                Ok(Some(old.2))
            }
        } else {
            if let Some(node_ptr) = self._free.pop_front() {
                unsafe {
                    let mut node_ptr = as_node::<MaybeUninit<(u64, K, V)>>(node_ptr);
                    let node = node_ptr.as_mut();
                    node.data_mut().write((hashcode, k, v));
                    bucket_ref.insert_first(node_ptr);
                    self._len += 1;
                    Ok(None)
                }
            } else {
                Err((k, v))
            }
        }
    }

    pub fn remove<Q>(&mut self, k: &Q) -> Option<V>
    where
        K: core::borrow::Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let (node, hashcode, mut bucket_ptr) = self._find_node(&k);
        let bucket_ref = unsafe { bucket_ptr.as_mut() };

        if let Some(mut node_ptr) = node {
            unsafe {
                bucket_ref.remove(node_ptr);

                let node_ref = node_ptr.as_mut();
                let (hc, k, v) = node_ref.data().assume_init_read();

                self._free.insert_first(node_ptr);
                self._len -= 1;
                Some(v)
            }
        } else {
            None
        }
    }

    pub fn remove_when<Q, HF, EF>(&mut self, k: &Q, hf: &mut HF, ef: &mut EF) -> Option<V>
    where
        HF: FnMut(&Q) -> Result<usize, ()>,
        EF: FnMut(&K, &Q) -> Result<bool, ()>,
    {
        let (node, hashcode, mut bucket_ptr) = self._find_node_when(k, hf, ef).ok()?;
        let bucket_ref = unsafe { bucket_ptr.as_mut() };

        if let Some(mut node_ptr) = node {
            unsafe {
                bucket_ref.remove(node_ptr);

                let node_ref = node_ptr.as_mut();
                let (hc, k, v) = node_ref.data().assume_init_read();

                self._free.insert_first(node_ptr);
                self._len -= 1;
                Some(v)
            }
        } else {
            None
        }
    }

    pub fn get<Q>(&self, k: &Q) -> Option<&V>
    where
        K: core::borrow::Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let (node, hashcode, bucket_ptr) = self._find_node(&k);
        let bucket_ref = unsafe { bucket_ptr.as_ref() };
        if let Some(mut node_ptr) = node {
            unsafe {
                let node_ref = node_ptr.as_mut();
                let (hc, k, v) = node_ref.data().assume_init_ref();
                Some(v)
            }
        } else {
            None
        }
    }

    pub fn get_key_value<Q>(&self, k: &Q) -> Option<(&K, &V)>
    where
        K: core::borrow::Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let (node, hashcode, bucket_ptr) = self._find_node(&k);
        let bucket_ref = unsafe { bucket_ptr.as_ref() };
        if let Some(mut node_ptr) = node {
            unsafe {
                let node_ref = node_ptr.as_mut();
                let (hc, k, v) = node_ref.data().assume_init_ref();
                Some((k, v))
            }
        } else {
            None
        }
    }

    pub fn get_when<Q, HF, EF>(&self, k: &Q, hf: &mut HF, ef: &mut EF) -> Option<&V>
    where
        HF: FnMut(&Q) -> Result<usize, ()>,
        EF: FnMut(&K, &Q) -> Result<bool, ()>,
    {
        let (node, hashcode, bucket_ptr) = self._find_node_when(k, hf, ef).ok()?;
        let bucket_ref = unsafe { bucket_ptr.as_ref() };
        if let Some(mut node_ptr) = node {
            unsafe {
                let node_ref = node_ptr.as_mut();
                let (hc, k, v) = node_ref.data().assume_init_ref();
                Some(v)
            }
        } else {
            None
        }
    }

    pub fn get_key_value_when<Q, HF, EF>(&self, k: &Q, hf: &mut HF, ef: &mut EF) -> Option<(&K, &V)>
    where
        HF: FnMut(&Q) -> Result<usize, ()>,
        EF: FnMut(&K, &Q) -> Result<bool, ()>,
    {
        let (node, hashcode, bucket_ptr) = self._find_node_when(k, hf, ef).ok()?;
        let bucket_ref = unsafe { bucket_ptr.as_ref() };
        if let Some(mut node_ptr) = node {
            unsafe {
                let node_ref = node_ptr.as_mut();
                let (hc, k, v) = node_ref.data().assume_init_ref();
                Some((k, v))
            }
        } else {
            None
        }
    }

    pub fn iter<'a>(&'a self) -> impl Iterator<Item = (&'a K, &'a V)> {
        unsafe {
            core::slice::from_raw_parts(self._data.as_ptr(), self._capacity)
                .iter()
                .map(|v| v.0.iter())
                .flatten()
                .map(|v| {
                    let (hc, k, v) = v.as_ref().data().assume_init_ref();
                    (k, v)
                })
        }
    }
}

impl<K, V> Drop for FixedMap<K, V> {
    fn drop(&mut self) {
        self.clear();
    }
}

#[repr(C)]
pub struct HashMap<K, V> {
    _allocator: &'static dyn Allocator,
    _inner_map: Option<NonNull<FixedMap<K, V>>>,
}

impl<K, V> Drop for HashMap<K, V> {
    fn drop(&mut self) {
        if let Some(inner) = self._inner_map {
            unsafe { FixedMap::<K, V>::free(self.allocator(), inner) };
            self._inner_map = None;
        }
    }
}

impl<K, V> HashMap<K, V> {
    #[cfg(debug_assertions)]
    fn debug_info(&self) -> Option<Debuginfo> {
        unsafe {
            if let Some(inner) = &self._inner_map {
                Some(inner.as_ref().debug_info())
            } else {
                None
            }
        }
    }

    pub fn new(allocator: &'static dyn Allocator) -> Self {
        Self {
            _allocator: allocator,
            _inner_map: None,
        }
    }

    fn allocator(&self) -> &dyn Allocator {
        self._allocator
    }

    pub fn capacity(&self) -> usize {
        unsafe {
            if let Some(inner) = &self._inner_map {
                inner.as_ref().capacity()
            } else {
                0
            }
        }
    }

    pub fn len(&self) -> usize {
        unsafe {
            if let Some(inner) = &self._inner_map {
                inner.as_ref().len()
            } else {
                0
            }
        }
    }

    pub fn reserve(&mut self, new_capacity: usize) -> Result<(), ()>
    where
        K: Hash + Eq,
    {
        if new_capacity <= self.capacity() {
            return Ok(());
        }

        let mut new_inner = FixedMap::<K, V>::new(self.allocator(), new_capacity).ok_or(())?;

        unsafe {
            if let Some(mut inner) = &self._inner_map {
                if let Err(()) = inner.as_mut().move_to(new_inner.as_mut()) {
                    FixedMap::<K, V>::free(self.allocator(), new_inner);
                }
            }
            if let Some(old) = self._inner_map.replace(new_inner) {
                FixedMap::<K, V>::free(self.allocator(), old);
            }
            Ok(())
        }
    }

    pub fn reserve_when<HF, EF>(
        &mut self,
        new_capacity: usize,
        hf: &mut HF,
        ef: &mut EF,
    ) -> Result<(), ()>
    where
        HF: FnMut(&K) -> Result<usize, ()>,
        EF: FnMut(&K, &K) -> Result<bool, ()>,
    {
        if new_capacity <= self.capacity() {
            return Ok(());
        }

        let mut new_inner = FixedMap::<K, V>::new(self.allocator(), new_capacity).ok_or(())?;

        unsafe {
            if let Some(mut inner) = &self._inner_map {
                if let Err(()) = inner.as_mut().move_to_when(new_inner.as_mut(), hf, ef) {
                    FixedMap::<K, V>::free(self.allocator(), new_inner);
                }
            }
            if let Some(old) = self._inner_map.replace(new_inner) {
                FixedMap::<K, V>::free(self.allocator(), old);
            }
            Ok(())
        }
    }

    pub fn insert(&mut self, key: K, value: V) -> Result<Option<V>, (K, V)>
    where
        K: Hash + Eq,
    {
        let cap = self.capacity();
        if cap - self.len() == 0 {
            let new_cap = if cap < 8 { 8 } else { cap * 2 };
            if self.reserve(new_cap).is_err() {
                return Err((key, value));
            }
        }

        if let Some(mut inner) = self._inner_map {
            unsafe { inner.as_mut().insert(key, value) }
        } else {
            Err((key, value))
        }
    }

    pub fn insert_when<HF, EF>(
        &mut self,
        key: K,
        value: V,
        hf: &mut HF,
        ef: &mut EF,
    ) -> Result<Option<V>, (K, V)>
    where
        HF: FnMut(&K) -> Result<usize, ()>,
        EF: FnMut(&K, &K) -> Result<bool, ()>,
    {
        let cap = self.capacity();
        if cap - self.len() == 0 {
            let new_cap = if cap < 8 { 8 } else { cap * 2 };
            if self.reserve_when(new_cap, hf, ef).is_err() {
                return Err((key, value));
            }
        }

        if let Some(mut inner) = self._inner_map {
            unsafe { inner.as_mut().insert_when(key, value, hf, ef) }
        } else {
            Err((key, value))
        }
    }

    pub fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: core::borrow::Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        if let Some(mut inner) = self._inner_map {
            unsafe { inner.as_mut().remove(key) }
        } else {
            None
        }
    }

    pub fn remove_when<Q, HF, EF>(&mut self, key: &Q, hf: &mut HF, ef: &mut EF) -> Option<V>
    where
        HF: FnMut(&Q) -> Result<usize, ()>,
        EF: FnMut(&K, &Q) -> Result<bool, ()>,
    {
        if let Some(mut inner) = self._inner_map {
            unsafe { inner.as_mut().remove_when(key, hf, ef) }
        } else {
            None
        }
    }

    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: core::borrow::Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        if let Some(inner) = self._inner_map {
            unsafe { inner.as_ref().get(key) }
        } else {
            None
        }
    }

    pub fn get_key_value<Q>(&self, key: &Q) -> Option<(&K, &V)>
    where
        K: core::borrow::Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        if let Some(inner) = self._inner_map {
            unsafe { inner.as_ref().get_key_value(key) }
        } else {
            None
        }
    }

    pub fn get_when<Q, HF, EF>(&self, key: &Q, hf: &mut HF, ef: &mut EF) -> Option<&V>
    where
        HF: FnMut(&Q) -> Result<usize, ()>,
        EF: FnMut(&K, &Q) -> Result<bool, ()>,
    {
        if let Some(inner) = self._inner_map {
            unsafe { inner.as_ref().get_when(key, hf, ef) }
        } else {
            None
        }
    }

    pub fn get_key_value_when<Q, HF, EF>(
        &self,
        key: &Q,
        hf: &mut HF,
        ef: &mut EF,
    ) -> Option<(&K, &V)>
    where
        HF: FnMut(&Q) -> Result<usize, ()>,
        EF: FnMut(&K, &Q) -> Result<bool, ()>,
    {
        if let Some(inner) = self._inner_map {
            unsafe { inner.as_ref().get_key_value_when(key, hf, ef) }
        } else {
            None
        }
    }

    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: core::borrow::Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        if let Some(inner) = self._inner_map {
            unsafe { inner.as_ref().get_key_value(key).is_some() }
        } else {
            false
        }
    }

    pub fn contains_key_when<Q, HF, EF>(&self, key: &Q, hf: &mut HF, ef: &mut EF) -> bool
    where
        HF: FnMut(&Q) -> Result<usize, ()>,
        EF: FnMut(&K, &Q) -> Result<bool, ()>,
    {
        if let Some(inner) = self._inner_map {
            unsafe { inner.as_ref().get_key_value_when(key, hf, ef).is_some() }
        } else {
            false
        }
    }

    pub fn iter<'a>(&'a self) -> impl Iterator<Item = (&K, &V)> {
        let iter = if let Some(inner) = self._inner_map {
            unsafe { Some(inner.as_ref().iter()) }
        } else {
            None
        };
        iter.into_iter().flatten()
    }
}

#[cfg(test)]
mod test {
    use super::super::super::alloc;
    use super::*;

    fn allocator() -> &'static dyn Allocator {
        alloc::default_allocator()
    }

    #[test]
    fn test_fixed_map() {
        let map_ptr = FixedMap::<i32, i32>::new(allocator(), 8);
        assert!(map_ptr.is_some());

        let mut map_ptr = map_ptr.unwrap();
        let map = unsafe { map_ptr.as_mut() };
        for i in 0..8 {
            assert!(map.insert(i, i * i).is_ok());
            assert_eq!(map.len(), (i + 1) as usize);
        }

        for i in 0..8 {
            assert!(map.get_key_value(&i).is_some());
            let (k, v) = map.get_key_value(&i).unwrap();
            assert_eq!(*k, i);
            assert_eq!(*v, i * i);
        }

        for i in 0..8 {
            assert_eq!(map.insert(i, i + 10).unwrap().unwrap(), i * i);
            assert_eq!(map.len(), 8);
        }

        assert_eq!(map.insert(9, 99).unwrap_err(), (9, 99));

        assert!(map.remove(&10).is_none());

        for i in 0..8 {
            assert_eq!(map.len(), (8 - i) as usize);
            assert_eq!(map.remove(&i).unwrap(), i + 10);
            assert_eq!(map.len(), (8 - i - 1) as usize);
        }

        assert!(map.remove(&10).is_none());

        unsafe { FixedMap::free(allocator(), map_ptr) };
    }

    #[test]
    fn test_map() {
        let mut map = HashMap::new(allocator());

        for i in 0..80 {
            assert!(map.insert(i, i * i).is_ok());
            assert_eq!(map.len(), (i + 1) as usize);
        }

        for i in 0..80 {
            assert!(map.get_key_value(&i).is_some());
            let (k, v) = map.get_key_value(&i).unwrap();
            assert_eq!(*k, i);
            assert_eq!(*v, i * i);
        }

        for i in 0..80 {
            assert_eq!(map.insert(i, i + 10).unwrap().unwrap(), i * i);
            assert_eq!(map.len(), 80);
        }

        assert_eq!(map.insert(99, 99).unwrap(), None);

        assert!(map.remove(&100).is_none());

        assert_eq!(map.len(), 81);

        let mut tl = map.len();
        for i in 0..80 {
            assert_eq!(map.len(), tl);
            assert_eq!(map.remove(&i).unwrap(), i + 10);
            tl -= 1;
            assert_eq!(map.len(), tl);
        }

        assert!(map.remove(&100).is_none());
    }

    #[test]
    fn test_map_iter() {
        let mut map = HashMap::new(allocator());
        for i in 0..4096 {
            map.insert(i.to_string(), (i * 2).to_string()).unwrap();
        }

        let mut v = map
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect::<Vec<_>>();

        v.sort_by(|a, b| {
            a.0.parse::<usize>()
                .unwrap()
                .cmp(&b.0.parse::<usize>().unwrap())
        });

        for i in 0..4096 {
            assert_eq!(*v[i].0, i.to_string());
            assert_eq!(*v[i].1, (i * 2).to_string());
        }

        #[cfg(debug_assertions)]
        {
            let info = map.debug_info();
            // println!("debug info: {:?}", info);
            // assert!(false);
        }
    }
}
