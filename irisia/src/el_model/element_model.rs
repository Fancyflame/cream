use std::{
    cell::Cell,
    future::Future,
    ops::{Deref, DerefMut},
    rc::Rc,
};

use tokio::task::JoinHandle;

use crate::{
    application::{
        content::GlobalContent,
        event_comp::{IncomingPointerEvent, NodeEventMgr},
    },
    data_flow::observer::{Observer, RcObserver},
    element::Render,
    event::{standard::ElementAbandoned, EventDispatcher, Listen},
    primitive::Region,
    structure::StructureCreate,
    ElementInterfaces, Result,
};

#[derive(Clone)]
pub struct ElementAccess(Rc<Shared>);

pub struct ElementModel<El, Cp> {
    pub(crate) el: El,
    pub(crate) event_mgr: NodeEventMgr,
    pub(crate) shared: Rc<Shared>,
    pub(crate) redraw_hook: RcObserver,
    pub(crate) child_props: Cp,
}

pub(crate) struct Shared {
    pub interact_region: Cell<Option<Region>>,
    pub draw_region: Cell<Region>,
    pub redraw_signal_sent: Cell<bool>,
    pub ed: EventDispatcher,
    pub gc: Rc<GlobalContent>,
}

#[derive(Clone)]
pub struct EMCreateCtx {
    pub(crate) global_content: Rc<GlobalContent>,
}

impl ElementAccess {
    pub fn interact_region(&self) -> Option<Region> {
        self.0.interact_region.get()
    }

    pub fn set_interact_region(&self, region: Option<Region>) {
        self.0.interact_region.set(region)
    }

    pub fn event_dispatcher(&self) -> &EventDispatcher {
        &self.0.ed
    }

    pub fn global_content(&self) -> &Rc<GlobalContent> {
        &self.0.gc
    }

    pub fn context(&self) -> EMCreateCtx {
        EMCreateCtx {
            global_content: self.0.gc.clone(),
        }
    }

    /// Listen event with options
    pub fn listen<Async, SubEv, WithHd>(
        &self,
    ) -> Listen<EventDispatcher, (), (), Async, SubEv, WithHd> {
        Listen::new(self.event_dispatcher())
    }

    pub fn request_redraw(&self) {
        self.0.request_redraw()
    }

    pub fn draw_region(&self) -> Region {
        self.0.draw_region.get()
    }
}

impl Shared {
    fn request_redraw(&self) {
        if self.redraw_signal_sent.get() {
            return;
        }

        self.gc.request_redraw(
            self.render_on
                .get_layer()
                .upgrade()
                .expect("parent rendering layer uninitialized or already dropped"),
        );

        self.redraw_signal_sent.set(true);
    }
}

impl<El, Cp> ElementModel<El, Cp> {
    pub(crate) fn new<Slt, Sty>(
        context: &EMCreateCtx,
        props: El::Props<'_>,
        child_props: Cp,
        slot: Slt,
    ) -> Self
    where
        El: ElementInterfaces,
        Slt: StructureCreate,
    {
        let ed = EventDispatcher::new();

        let shared = Rc::new(Shared {
            interact_region: Cell::new(None),
            draw_region: Default::default(),
            redraw_signal_sent: Cell::new(false),
            ed: ed.clone(),
            gc: context.global_content.clone(),
        });

        ElementModel {
            el: El::create(props, slot, ElementAccess(shared.clone()), &context),
            event_mgr: NodeEventMgr::new(ed.clone()).into(),
            redraw_hook: {
                let shared = shared.clone();
                Observer::new(move || shared.request_redraw())
            },
            shared,
            child_props,
        }
    }

    pub(crate) fn set_draw_region(&self, region: Region)
    where
        El: ElementInterfaces,
    {
        self.shared.draw_region.set(region);
        self.el.borrow_mut().set_draw_region(region);
    }

    /// Get event dispatcher of this element.
    pub fn event_dispatcher(&self) -> &EventDispatcher {
        &self.shared.ed
    }

    /// Let this element being focused on.
    pub fn focus(&self) {
        self.global_content()
            .focusing()
            .focus(self.event_dispatcher().clone());
    }

    /// Let this element no longer being focused. does nothing if
    /// this element is not in focus.
    pub fn blur(&self) {
        self.global_content()
            .focusing()
            .blur_checked(&self.event_dispatcher());
    }

    /// Get global content of the window.
    pub fn global_content(&self) -> &Rc<GlobalContent> {
        &self.shared.gc
    }

    pub fn set_interact_region(&self, region: Option<Region>) {
        self.shared.interact_region.set(region);
    }

    pub fn interact_region(&self) -> Option<Region> {
        self.shared.interact_region.get()
    }

    pub fn request_redraw(&self) {
        self.shared.request_redraw()
    }

    /// Spwan a daemon task on `fut`.
    ///
    /// The spawned task will be cancelled when element dropped,
    /// or can be cancelled manually.
    pub fn daemon<F>(&self, fut: F) -> JoinHandle<()>
    where
        F: Future + 'static,
    {
        let ed = self.event_dispatcher().clone();
        tokio::task::spawn_local(async move {
            tokio::select! {
                _ = ed.recv_trusted::<ElementAbandoned>() => {},
                _ = fut => {}
            }
        })
    }

    pub fn access(&self) -> ElementAccess {
        ElementAccess(self.shared.clone())
    }

    /// returns whether this element is logically entered
    pub fn on_pointer_event(&mut self, ipe: &IncomingPointerEvent) -> bool
    where
        El: ElementInterfaces,
    {
        let children_logically_entered = self.el.borrow_mut().children_emit_event(ipe);
        self.event_mgr.update_and_emit(
            ipe,
            self.shared.interact_region.get(),
            children_logically_entered,
        )
    }

    pub(crate) fn monitoring_render(&mut self, args: Render) -> Result<()>
    where
        El: ElementInterfaces,
    {
        self.redraw_hook
            .invoke(|| self.el.get_mut().unwrap().render(args))
    }
}

impl<El, Cp> Drop for ElementModel<El, Cp> {
    fn drop(&mut self) {
        self.shared.ed.emit_trusted(ElementAbandoned)
    }
}

pub(crate) struct InitLater<T>(pub(super) Option<T>);

impl<T> Deref for InitLater<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.0.as_ref().unwrap()
    }
}

impl<T> DerefMut for InitLater<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_mut().unwrap()
    }
}
