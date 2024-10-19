use std::{
    cell::RefCell,
    collections::{hash_map::Entry, HashMap},
};

use crate::{
    data_flow::{dirty_flag::DirtyFlag, ReadWire, Readable, ToReadWire},
    el_model::EMCreateCtx,
};

use super::{StructureCreate, VisitBy};

pub struct Repeat<Wire, Tb>
where
    Tb: TreeBuilderFn<Wire>,
{
    src: ReadWire<Vec<Wire>>,
    dirty_flag: DirtyFlag,
    tree_builder: Tb,
    map: RefCell<HashMap<*const (), MapItem<Wire, Tb::Tree>>>,
    ctx: EMCreateCtx,
}

struct MapItem<Wire, Tree> {
    _data: Wire,
    tree: Tree,
    alive: bool,
}

impl<Cp, Wire, Tb> VisitBy<Cp> for Repeat<Wire, Tb>
where
    Wire: Readable + Clone + 'static,
    Tb: TreeBuilderFn<Wire>,
    Tb::Tree: VisitBy<Cp>,
{
    fn visit<V>(&self, v: &mut V) -> crate::Result<()>
    where
        V: super::Visitor<Cp>,
    {
        self.update_if_need();
        let map = self.map.borrow();
        for data in &*self.src.read() {
            map[&data.ptr_as_id()].tree.visit(v)?;
        }
        Ok(())
    }

    fn visit_mut<V>(&mut self, v: &mut V) -> crate::Result<()>
    where
        V: super::VisitorMut<Cp>,
    {
        self.update_if_need();
        let mut map = self.map.borrow_mut();
        for data in &*self.src.read() {
            map.get_mut(&data.ptr_as_id()).unwrap().tree.visit_mut(v)?;
        }
        Ok(())
    }
}

impl<Wire, Tb> Repeat<Wire, Tb>
where
    Wire: Readable + Clone,
    Tb: TreeBuilderFn<Wire>,
{
    fn update_if_need(&self) {
        if !self.dirty_flag.is_dirty() {
            return;
        }

        let mut map = self.map.borrow_mut();

        for wire in &*self.src.read() {
            match map.entry(wire.ptr_as_id()) {
                Entry::Occupied(mut occ) => {
                    occ.get_mut().alive = true;
                }
                Entry::Vacant(vac) => {
                    vac.insert(MapItem {
                        _data: wire.clone(),
                        tree: self.tree_builder.build(wire, &self.ctx),
                        alive: true,
                    });
                }
            }
        }

        map.retain(|_, item| {
            let alive = item.alive;
            if alive {
                item.alive = false;
            }
            alive
        });

        self.dirty_flag.set_clean();
    }
}

pub fn repeat<V, W, F, Tb>(vec: V, body: F) -> impl StructureCreate
where
    V: ToReadWire<Data = Vec<W>>,
    W: ToReadWire + 'static,
    F: Fn(&W) -> Tb + 'static,
    Tb: StructureCreate + 'static,
{
    let vec = vec.to_read_wire();
    |ctx: &EMCreateCtx| Repeat {
        dirty_flag: {
            let flag = DirtyFlag::new();
            flag.set_dirty();
            vec.add_listener(&flag);
            flag
        },
        src: vec,
        tree_builder: body,
        map: RefCell::new(HashMap::new()),
        ctx: ctx.clone(),
    }
}

pub trait TreeBuilderFn<W>: 'static {
    type Tree;
    fn build(&self, input_wire: &W, ctx: &EMCreateCtx) -> Self::Tree;
}

impl<F, W, Tb> TreeBuilderFn<W> for F
where
    F: Fn(&W) -> Tb + 'static,
    Tb: StructureCreate,
{
    type Tree = Tb::Target;
    fn build(&self, input_wire: &W, ctx: &EMCreateCtx) -> Self::Tree {
        self(input_wire).create(ctx)
    }
}
