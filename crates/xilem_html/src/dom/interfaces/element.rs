use std::borrow::Cow;

use wasm_bindgen::JsCast;

use crate::{
    dom::{attribute::Attr, elements::ElementState, event::EventListener},
    view::DomElement,
    IntoAttributeValue, OptionalAction, View, ViewMarker,
};

use super::Node;
pub trait Element<T, A = ()>: Node + View<T, A> + ViewMarker
where
    Self: Sized,
{
    // TODO should the API be "functional" in the sense, that new attributes are wrappers around the type,
    // or should they modify the underlying instance (e.g. via the following methods)?
    // The disadvantage that "functional" brings in, is that elements are not modifiable (i.e. attributes can't be simply added etc.)
    // fn attrs(&self) -> &Attributes;
    // fn attrs_mut(&mut self) -> &mut Attributes;

    /// Set an attribute on this element.
    ///
    /// # Panics
    ///
    /// If the name contains characters that are not valid in an attribute name,
    /// then the `View::build`/`View::rebuild` functions will panic for this view.
    fn attr<K: Into<Cow<'static, str>>, V: IntoAttributeValue>(
        self,
        name: K,
        value: V,
    ) -> Attr<Self> {
        Attr {
            element: self,
            name: name.into(),
            value: value.into_attribute_value(),
        }
    }
}

impl<T, A, E: Element<T, A>, ES> Element<T, A> for Attr<E>
where
    E: View<T, A, State = ElementState<ES>>,
    E::Element: DomElement,
{
}

impl<T, A, E, ES, Ev, F, OA> Element<T, A> for EventListener<E, Ev, F>
where
    F: Fn(&mut T, Ev) -> OA,
    E: View<T, A, State = ElementState<ES>> + Element<T, A>,
    Ev: JsCast + 'static,
    OA: OptionalAction<A>,
{
}
