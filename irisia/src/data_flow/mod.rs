use std::{ops::Deref, rc::Rc};

pub use self::{
    const_data::const_wire,
    register::Register,
    watcher::Watcher,
    wire::{wire, wire2, wire3, wire_cmp},
};
use deps::{Listener, ListenerList};
use map::Map;
use trace_cell::TraceRef;

pub mod const_data;
mod deps;
pub mod dirty_flag;
pub mod map;
pub mod observer;
pub mod register;
pub mod trace_cell;
pub mod watch_on_deref;
pub mod watcher;
pub mod wire;

pub type ReadWire<T> = Rc<dyn Readable<Data = T>>;

pub trait Readable: Listenable {
    type Data: ?Sized;
    fn read(&self) -> ReadRef<Self::Data>;
}

pub trait Listenable {
    fn add_listener(&self, listener: &Listener);
    fn remove_listener(&self, listener: &Listener);
}

pub trait Wakeable {
    fn add_back_reference(&self, dep: &Rc<dyn Listenable>);
    fn set_dirty(&self);
    fn wake(&self) -> bool;
}

pub trait ReadableExt: Readable + 'static {
    fn watch<F>(&self, callback: F) -> Watcher
    where
        F: FnMut() -> bool + 'static,
    {
        let w = Watcher::new(callback);
        w.watch(self);
        w
    }

    fn map<R>(self, f: fn(&Self::Data) -> &R) -> Map<Self, Self::Data, R>
    where
        Self: Sized,
    {
        Map::new(self, f)
    }
}

impl<T> ReadableExt for T where T: Readable + ?Sized + 'static {}

impl<T> Readable for Rc<T>
where
    T: Readable + ?Sized,
{
    type Data = T::Data;

    fn read(&self) -> ReadRef<Self::Data> {
        (**self).read()
    }
}

impl<T> Listenable for Rc<T>
where
    T: Listenable + ?Sized,
{
    fn add_listener(&self, listener: &Listener) {
        (**self).add_listener(listener);
    }

    fn remove_listener(&self, listener: &Listener) {
        (**self).remove_listener(listener);
    }
}

pub enum ReadRef<'a, T>
where
    T: ?Sized,
{
    Ref(&'a T),
    CellRef(TraceRef<'a, T>),
}

impl<T> Deref for ReadRef<'_, T>
where
    T: ?Sized,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self {
            ReadRef::Ref(r) => r,
            ReadRef::CellRef(r) => r,
        }
    }
}

impl<'a, T: ?Sized> ReadRef<'a, T> {
    pub fn clone(this: &Self) -> ReadRef<'a, T> {
        match this {
            ReadRef::Ref(r) => ReadRef::Ref(r),
            ReadRef::CellRef(r) => ReadRef::CellRef(TraceRef::clone(r)),
        }
    }

    pub fn map<U, F>(self, f: F) -> ReadRef<'a, U>
    where
        F: FnOnce(&T) -> &U,
        U: ?Sized,
    {
        match self {
            ReadRef::Ref(r) => ReadRef::Ref(f(r)),
            ReadRef::CellRef(r) => ReadRef::CellRef(TraceRef::map(r, f)),
        }
    }
}

pub trait AsReadWire {
    type TargetData: ?Sized;
    fn as_read_wire(&self) -> &ReadWire<Self::TargetData>;
}

impl<T> AsReadWire for Rc<T>
where
    T: Readable,
{
    type TargetData = <Self as Readable>::Data;
    fn as_read_wire(&self) -> &ReadWire<Self::TargetData> {
        self
    }
}

impl<Data: ?Sized> AsReadWire for ReadWire<Data> {
    type TargetData = Data;
    fn as_read_wire(&self) -> &ReadWire<Self::TargetData> {
        self
    }
}
