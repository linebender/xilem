use std::borrow::Cow;

pub trait SetAttr {
    fn set_attr(&mut self, name: impl Into<Cow<'static, str>>, value: impl Into<Cow<'static, str>>);

    fn attr(
        mut self,
        name: impl Into<Cow<'static, str>>,
        value: impl Into<Cow<'static, str>>,
    ) -> Self
    where
        Self: Sized,
    {
        self.set_attr(name, value);
        self
    }
}
