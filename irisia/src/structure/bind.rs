use std::cell::Cell;

use crate::__private::dep_stack::Bitset;

use super::{tracert::TracertBase, StructureUpdateTo, Updating, VisitBy};

pub struct Bind<T, const WD: usize> {
    data: T,
    dependents: Cell<Bitset<WD>>,
}

pub struct BindUpdater<F>(pub F);

impl<T, const WD: usize> VisitBy for Bind<T, WD>
where
    T: VisitBy,
{
    fn visit_by<V>(&self, visitor: &mut V) -> crate::Result<()>
    where
        V: super::VisitOn,
    {
        self.data.visit_by(visitor)
    }

    fn len(&self) -> usize {
        self.data.len()
    }

    fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

impl<T, F, Tu, const WD: usize> StructureUpdateTo<WD> for BindUpdater<F>
where
    Self: VisitBy,
    F: for<'a> FnOnce(TracertBase<'a, WD>) -> Tu,
    Tu: StructureUpdateTo<WD, Target = T>,
{
    type Target = Bind<T, WD>;
    const UPDATE_POINTS: u32 = 1 + Tu::UPDATE_POINTS;

    fn create(self, mut info: Updating<WD>) -> Self::Target {
        let dependents = Cell::new(Bitset::new());
        let child_updater = info.scoped(0, || (self.0)(TracertBase::new(info.stack, &dependents)));

        Bind {
            dependents,
            data: child_updater.create(info.inherit(1, false)),
        }
    }

    fn update(self, target: &mut Self::Target, mut info: Updating<WD>) {
        if info.no_update::<Self>() {
            return;
        }

        info.step_if(0);

        info.points.union(&target.dependents.take());
        let child_updater = info.scoped(0, || {
            (self.0)(TracertBase::new(&info.stack, &target.dependents))
        });
        child_updater.update(&mut target.data, info.inherit(1, false));
    }
}
