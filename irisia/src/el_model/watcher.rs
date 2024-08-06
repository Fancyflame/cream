use std::rc::Weak;

use crate::{
    data_flow::{ReadWire, ReadableExt},
    ElementInterfaces,
};

use super::ElementModel;

pub struct ElInputWatcher<El>(Weak<ElementModel<El>>);

impl<El> Clone for ElInputWatcher<El> {
    fn clone(&self) -> Self {
        ElInputWatcher(self.0.clone())
    }
}

impl<El: ElementInterfaces> ElInputWatcher<El> {
    pub(crate) fn new(em: Weak<ElementModel<El>>) -> Self {
        ElInputWatcher(em)
    }

    pub fn invoke<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&El) -> R,
    {
        let Some(rc) = self.0.upgrade() else {
            panic!("cannot invoke function on a dropped element");
        };

        let r = f(&rc.el.borrow());
        r
    }

    pub fn invoke_mut<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut El) -> R,
    {
        let Some(rc) = self.0.upgrade() else {
            panic!("cannot invoke function on a dropped element");
        };

        let r = f(&mut rc.el.borrow_mut());
        r
    }

    pub fn watch<U, F>(&self, watch: ReadWire<U>, mut func: F)
    where
        U: 'static,
        F: FnMut(&mut El, &U) + 'static,
    {
        let em = self.0.clone();
        watch.watch(
            move |wire, handle| {
                let Some(em) = em.upgrade() else {
                    handle.discard();
                    return;
                };

                if let Some(el) = &mut em.el.borrow_mut().0 {
                    func(el, &wire.r());
                };
            },
            false,
        );
    }
}
