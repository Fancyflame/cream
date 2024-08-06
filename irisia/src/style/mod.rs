pub use self::read_style::{ReadStyle, StyleBuffer};

mod read_style;

pub trait Style: Clone + 'static {}

pub trait WriteStyle {
    fn write_style<R>(&mut self, read: &R)
    where
        R: ReadStyle + ?Sized;

    #[inline]
    fn from_style<R>(read: &R) -> Self
    where
        Self: Default,
        R: ReadStyle + ?Sized,
    {
        let mut this = Self::default();
        this.write_style(read);
        this
    }
}

impl<T> WriteStyle for Option<T>
where
    T: Style,
{
    #[inline]
    fn write_style<R>(&mut self, read: &R)
    where
        R: ReadStyle + ?Sized,
    {
        read.read_style_into(&mut StyleBuffer(self));
    }
}

impl<T> WriteStyle for T
where
    T: Style,
{
    #[inline]
    fn write_style<R>(&mut self, read: &R)
    where
        R: ReadStyle + ?Sized,
    {
        let mut option = None;
        read.read_style_into(&mut StyleBuffer(&mut option));
        if let Some(value) = option {
            *self = value;
        }
    }
}

pub trait StyleFn<Tuple>: Style {
    fn from(tuple: Tuple) -> Self;
}

impl<T: Style> StyleFn<(Self,)> for T {
    fn from(tuple: (Self,)) -> Self {
        tuple.0
    }
}
