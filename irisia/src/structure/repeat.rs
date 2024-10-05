use std::{
    collections::{hash_map::Entry, HashMap},
    hash::Hash,
    rc::Rc,
};

use crate::{
    data_flow::{
        register::{register, Register},
        wire3, ReadWire,
    },
    el_model::EMCreateCtx,
};

use super::{StructureCreate, VisitBy};

const MAX_TIME_TO_LIVE: u8 = 3;

pub struct Repeat<K, T, Tree> {
    order: ReadWire<Vec<K>>,
    //dirty:
    map: HashMap<K, Item<T, Tree>>,
    ctx: EMCreateCtx,
}

struct Item<T, Tree> {
    iter_item: Rc<Register<T>>,
    tree: Tree,
    time_to_live: u8,
}

impl<Cp, K, T, Tree> VisitBy<Cp> for Repeat<K, T, Tree>
where
    K: Hash + Eq + 'static,
    T: 'static,
    Tree: VisitBy<Cp>,
{
    fn visit<V>(&self, v: &mut V) -> crate::Result<()>
    where
        V: super::Visitor,
    {
        let this = self.0.read();

        for key in &this.order {
            this.map[key].tree.visit(v)?;
        }

        Ok(())
    }

    fn visit_mut<V>(&mut self, v: &mut V) -> crate::Result<()>
    where
        V: super::Visitor,
    {
        let this = self.0.read();

        for key in &this.order {
            this.map.get_mut(key).unwrap().tree.visit_mut(v)?;
        }

        Ok(())
    }
}

pub struct RepeatMutator<'a, K, T, Tree>(&'a mut RepeatInner<K, T, Tree>);

impl<'a, K, Item, Tree> RepeatMutator<'a, K, Item, Tree>
where
    K: Hash + Eq + Clone,
    Item: 'static,
{
    pub fn update<Iter, Fk, F, Upd>(self, iter: Iter, key_fn: Fk, content_fn: F)
    where
        Iter: IntoIterator<Item = Item>,
        Fk: Fn(&Item) -> K,
        F: Fn(ReadWire<Item>) -> Upd,
        Upd: StructureCreate<Target = Tree>,
    {
        self.0.update(
            iter.into_iter().map(|data| (key_fn(&data), data)),
            content_fn,
        );
    }
}

pub fn repeat<K, T, Tree, F>(content_fn: F) -> impl StructureCreate
where
    K: Hash + Eq + Clone + 'static,
    T: 'static,
    Tree: VisitBy,
    F: Fn(RepeatMutator<K, T, Tree>) + 'static,
{
    move |ctx: &EMCreateCtx| {
        let ctx = ctx.clone();

        let w = wire3(
            move || {
                let rep = RepeatInner {
                    map: HashMap::new(),
                    ctx,
                    order: Vec::new(),
                };

                (rep, move |mut r| content_fn(RepeatMutator(&mut r)))
            },
            true,
        );

        Repeat(w)
    }
}

impl<K, T, Tree> RepeatInner<K, T, Tree> {
    fn update<I, Upd, F>(&mut self, iter: I, content_fn: F)
    where
        K: Hash + Eq + Clone,
        T: 'static,
        I: Iterator<Item = (K, T)>,
        F: Fn(ReadWire<T>) -> Upd,
        Upd: StructureCreate<Target = Tree>,
    {
        let RepeatInner { map, order, ctx } = self;

        order.clear();
        for (key, data) in iter {
            match map.entry(key.clone()) {
                Entry::Occupied(mut occ) => {
                    let item = occ.get_mut();
                    assert_ne!(
                        item.time_to_live, MAX_TIME_TO_LIVE,
                        "some keys in the iterator is duplicated"
                    );
                    item.time_to_live = MAX_TIME_TO_LIVE;
                    item.iter_item.set(data);
                }
                Entry::Vacant(vac) => {
                    let reg = register(data);
                    vac.insert(Item {
                        tree: content_fn(reg.clone()).create(ctx),
                        iter_item: reg,
                        time_to_live: MAX_TIME_TO_LIVE,
                    });
                }
            }

            order.push(key);
        }

        map.retain(|_, item| match item.time_to_live.checked_sub(1) {
            Some(ttl) => {
                item.time_to_live = ttl;
                true
            }
            None => false,
        });
    }
}
