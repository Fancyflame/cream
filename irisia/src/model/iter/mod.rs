use crate::el_model::ElementModel;
pub use basic::{ModelBasic, ModelBasicMapper};

pub mod basic;

pub trait VisitModel<M: ModelMapper> {
    fn visit(&self, f: &mut dyn FnMut(M::MapRef<'_>));
    fn visit_mut(&self, f: &mut dyn FnMut(M::MapMut<'_>));
}

pub trait ModelMapper: 'static {
    type MapRef<'a>;
    type MapMut<'a>;
}

pub trait ModelMapperImplements<El, Cp>: ModelMapper {
    fn map_ref(model: &ElementModel<El, Cp>) -> Self::MapRef<'_>;
    fn map_mut(model: &mut ElementModel<El, Cp>) -> Self::MapMut<'_>;
}
