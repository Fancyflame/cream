use std::{
    cell::Cell,
    ops::{Deref, DerefMut},
    rc::{Rc, Weak},
};

use super::{
    deps::DepdencyList,
    trace_cell::{TraceCell, TraceRef},
    Listenable, Listener, ListenerList, ReadRef, ReadWire, Readable, Wakeable,
};

const BORROW_ERROR: &str = "cannot update data inside the wire, because at least one reader still exists \
    or the last update operation has not completed. if it's because of the latter, it declares a wire \
    loop was detected, which means invoking the update function of this wire needs to read the \
    old data of this wire itself, which bound to cause infinite updating. to address this problem, \
    you should remove the loop manually";

pub fn wire<F, T>(f: F) -> ReadWire<T>
where
    F: Fn() -> T + 'static,
{
    Wire::new(move || {
        (f(), move |r| {
            *r = f();
            true
        })
    })
}

pub fn wire_cmp<F, T>(f: F) -> ReadWire<T>
where
    F: Fn() -> T + 'static,
    T: Eq,
{
    Wire::new(move || {
        (f(), move |r| {
            let value = f();
            let mutated = value != *r;
            *r = value;
            mutated
        })
    })
}

pub fn wire2<Fi, F, T>(init_state: Fi, f: F) -> ReadWire<T>
where
    Fi: FnOnce() -> T,
    F: Fn(Setter<T>) + 'static,
{
    wire3(move || (init_state(), f))
}

pub fn wire3<Fi, T, F>(fn_init: Fi) -> ReadWire<T>
where
    Fi: FnOnce() -> (T, F),
    F: Fn(Setter<T>),
{
    Wire::new(move || {
        let (init, updater) = fn_init();
        (init, move |r| {
            let mut mutated = false;
            updater(Setter {
                r,
                mutated: &mut mutated,
            });
            mutated
        })
    })
}

struct Wire<F, T> {
    computes: TraceCell<Option<(F, T)>>,
    listeners: ListenerList,
    deps: DepdencyList,
    as_listenable: Weak<dyn Listenable>,
    is_dirty: Cell<bool>,
}

impl<F, T> Wire<F, T>
where
    F: Fn(&mut T) -> bool,
{
    fn new<Fi>(fn_init: Fi) -> Rc<Self>
    where
        Fi: FnOnce() -> (T, F),
    {
        let w = Rc::new_cyclic(move |this| Wire {
            computes: TraceCell::new(None),
            listeners: ListenerList::new(),
            deps: DepdencyList::new(Listener::Weak(this.clone())),
            as_listenable: this.clone(),
            is_dirty: Cell::new(true),
        });

        let mut computes = w.computes.borrow_mut().unwrap();
        let init = w.deps.collect_dependencies(fn_init);
        *computes = Some((init.1, init.0));
        w
    }
}

impl<F, T> Readable for Wire<F, T>
where
    F: Fn(&mut T) -> bool,
{
    type Data = T;

    fn read(&self) -> ReadRef<Self::Data> {
        self.listeners
            .capture_caller(&self.as_listenable.upgrade().unwrap());
        ReadRef::CellRef(TraceRef::map(
            self.computes.borrow().expect(BORROW_ERROR),
            |(_, cache)| cache.as_ref().unwrap(),
        ))
    }
}

impl<F, T> Wakeable for Wire<F, T>
where
    F: Fn(&mut T) -> bool,
{
    fn add_back_reference(&self, dep: &Rc<dyn Listenable>) {
        self.deps.add_dependency(dep);
    }

    fn set_dirty(&self) {
        self.is_dirty.set(true);
        self.listeners.set_dirty();
    }

    fn wake(&self) -> bool {
        if !self.listeners.is_dirty() {
            return;
        }

        let mut computes = self.computes.borrow_mut().expect(BORROW_ERROR);
        let mut mutated = self
            .deps
            .collect_dependencies(|| (computes.0)(&mut computes.1));

        if mutated {
            self.listeners.wake_all();
        }

        self.is_dirty.set(false);
        true
    }
}

impl<F, T> Listenable for Wire<F, T> {
    fn add_listener(&self, listener: &Listener) {
        self.listeners.add_listener(listener);
    }

    fn remove_listener(&self, listener: &Listener) {
        self.listeners.remove_listener(listener);
    }
}

pub struct Setter<'a, T: ?Sized> {
    r: &'a mut T,
    mutated: &'a mut bool,
}

impl<T: ?Sized> Deref for Setter<'_, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.r
    }
}

impl<T: ?Sized> DerefMut for Setter<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        *self.mutated = true;
        self.r
    }
}
