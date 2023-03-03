use core::marker::PhantomData;
use core::mem::transmute;
use core::ptr::NonNull;
use core::ptr::{addr_of, addr_of_mut};

use crate::nonnull_of;

/// 该类型必须放在C样式类型的头部。
/// 并且要保证该类型的实例在其存活期间的内存不变。
#[repr(C)]
pub struct ListNodeBase {
    pub(self) _prev: NonNull<Self>,
    pub(self) _next: NonNull<Self>,
}

impl ListNodeBase {
    pub unsafe fn init(mut ptr: NonNull<Self>) {
        let r = ptr.as_mut();
        // addr_of_mut!(r._data).write(data);
        addr_of_mut!(r._prev).write(ptr);
        addr_of_mut!(r._next).write(ptr);
    }

    unsafe fn _clear_node(&mut self) {
        self._prev = NonNull::new_unchecked(self as *mut Self);
        self._next = NonNull::new_unchecked(self as *mut Self);
    }
}

#[inline]
pub unsafe fn as_node<T>(ptr: NonNull<ListNodeBase>) -> NonNull<ListNode<T>> {
    transmute(ptr)
}

#[repr(C)]
pub struct ListNode<T> {
    pub(self) _base: ListNodeBase,
    pub(self) _data: T,
}

impl<T> ListNode<T> {
    pub unsafe fn init(mut ptr: NonNull<Self>, data: T) {
        let base = nonnull_of!(ptr.as_mut()._base);
        ListNodeBase::init(base);

        addr_of_mut!(ptr.as_mut()._data).write(data);
    }

    unsafe fn _clear_node(&mut self) {
        let base = nonnull_of!(self._base);
        ListNodeBase::init(base);
    }

    #[inline]
    pub fn data(&self) -> &T {
        &self._data
    }
    #[inline]
    pub fn data_mut(&mut self) -> &mut T {
        &mut self._data
    }
}

pub trait ToListNode {
    fn to_base(self) -> NonNull<ListNodeBase>;
}

impl ToListNode for NonNull<ListNodeBase> {
    fn to_base(self) -> NonNull<ListNodeBase> {
        self
    }
}
impl<T> ToListNode for NonNull<ListNode<T>> {
    fn to_base(mut self) -> NonNull<ListNodeBase> {
        nonnull_of!(self.as_mut()._base)
    }
}

#[repr(C)]
pub struct ListBase {
    _guard: ListNodeBase,
    _len: usize,
}

impl ListBase {
    pub unsafe fn init(mut ptr: NonNull<Self>) {
        let r = ptr.as_mut();
        addr_of_mut!(r._len).write(0);

        ListNodeBase::init(nonnull_of!(r._guard));

        let guard_ptr = r._as_guard_ptr();
        let guard_ref = r._as_guard_mut();
        guard_ref._prev = guard_ptr;
        guard_ref._next = guard_ptr;
    }

    #[inline]
    fn _as_guard_ptr(&self) -> NonNull<ListNodeBase> {
        unsafe {
            let ptr = addr_of!(self._guard) as _;
            NonNull::new_unchecked(ptr)
        }
    }

    #[inline]
    fn _as_guard(&self) -> &ListNodeBase {
        unsafe {
            let ptr = addr_of!(self._guard) as *const ListNodeBase;
            &*ptr
        }
    }

    #[inline]
    fn _as_guard_mut(&mut self) -> &mut ListNodeBase {
        unsafe {
            let ptr = addr_of_mut!(self._guard) as *mut ListNodeBase;
            &mut *ptr
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self._len
    }

    #[inline]
    pub fn insert_first<T: ToListNode>(&mut self, node: T) {
        unsafe {
            let mut node = node.to_base();
            let node_ref = node.as_mut();
            let guard_ptr = self._as_guard_ptr();
            let guard_ref = self._as_guard_mut();
            let mut old_next = guard_ref._next;

            old_next.as_mut()._prev = node;
            guard_ref._next = node;
            node_ref._prev = guard_ptr;
            node_ref._next = old_next;

            self._len += 1;
        }
    }

    #[inline]
    pub fn insert_last<T: ToListNode>(&mut self, node: T) {
        unsafe {
            let mut node = node.to_base();
            let node_ref = node.as_mut();
            let guard_ptr = self._as_guard_ptr();
            let guard_ref = self._as_guard_mut();
            let mut old_prev = guard_ref._prev;

            guard_ref._prev = node;
            old_prev.as_mut()._next = node;
            node_ref._prev = old_prev;
            node_ref._next = guard_ptr;

            self._len += 1;
        }
    }

    #[inline]
    pub fn remove<T: ToListNode>(&mut self, node: T) {
        unsafe {
            let mut node = node.to_base();
            let node_ref = node.as_mut();
            let mut old_prev = node_ref._prev;
            let mut old_next = node_ref._next;

            node_ref._clear_node();

            old_next.as_mut()._prev = old_prev;
            old_prev.as_mut()._next = old_next;

            self._len -= 1;
        }
    }

    #[inline]
    pub fn pop_front(&mut self) -> Option<NonNull<ListNodeBase>> {
        let guard_ptr = self._as_guard_ptr();
        let guard_ref = self._as_guard_mut();
        let first = guard_ref._next;
        if first != guard_ptr {
            self.remove(first);
            Some(first)
        } else {
            None
        }
    }

    #[inline]
    pub fn pop_back(&mut self) -> Option<NonNull<ListNodeBase>> {
        let guard_ptr = self._as_guard_ptr();
        let guard_ref = self._as_guard_mut();
        let last = guard_ref._prev;
        if last != guard_ptr {
            self.remove(last);
            Some(last)
        } else {
            None
        }
    }

    pub fn iter(&self) -> ListIter {
        ListIter::new(self)
    }
}

#[repr(C)]
pub struct List<T> {
    _list: ListBase,
    _phantom: PhantomData<T>,
}

impl<T> List<T> {
    pub unsafe fn init(mut ptr: NonNull<Self>) {
        let base_ptr = nonnull_of!(ptr.as_mut()._list);
        ListBase::init(base_ptr);
    }

    #[inline]
    pub fn len(&self) -> usize {
        self._list._len
    }

    #[inline]
    pub fn insert_first(&mut self, node: NonNull<ListNode<T>>) {
        self._list.insert_first(node)
    }

    #[inline]
    pub fn insert_last(&mut self, node: NonNull<ListNode<T>>) {
        self._list.insert_last(node)
    }

    #[inline]
    pub fn remove(&mut self, node: NonNull<ListNode<T>>) {
        self._list.remove(node)
    }

    #[inline]
    pub fn pop_front(&mut self) -> Option<NonNull<ListNode<T>>> {
        unsafe { self._list.pop_front().map(|n| as_node(n)) }
    }

    #[inline]
    pub fn pop_back(&mut self) -> Option<NonNull<ListNode<T>>> {
        unsafe { self._list.pop_back().map(|n| as_node(n)) }
    }

    pub fn iter(&self) -> impl Iterator<Item = NonNull<ListNode<T>>> {
        self._list.iter().map(|node| unsafe { as_node::<T>(node) })
    }
}

pub struct ListIter {
    _guard: NonNull<ListNodeBase>,
    _current: NonNull<ListNodeBase>,
}

impl ListIter {
    pub fn new(list: &ListBase) -> Self {
        let guard_ref = list._as_guard();
        let guard_ptr = list._as_guard_ptr();
        Self {
            _guard: guard_ptr,
            _current: guard_ref._next,
        }
    }
}

impl Iterator for ListIter {
    type Item = NonNull<ListNodeBase>;
    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            if self._current != self._guard {
                let v = self._current;
                self._current = self._current.as_mut()._next;
                Some(v)
            } else {
                None
            }
        }
    }
}
