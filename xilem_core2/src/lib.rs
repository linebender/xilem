use std::any::Any;

/// Needed:
/// 1) View trait
/// 2) ViewSequence trait
/// 3) AnyView trait
///
/// View trait has an element type
/// AnyView trait is implemented for any sequence which can
/// ViewSequence trait

pub trait View<State, Action> {
    type ViewState;
    type Element: Element;
}

pub trait Element {
    type Mut<'a>;
}

pub trait SuperElement<Child>: Element
where
    Child: Element,
{
    fn upcast(child: Child) -> Self;
    fn downcast<'a>(refm: Self::Mut<'a>) -> Child::Mut<'a>;
}

// TODO: What do we want to do here? This impl seems nice, but is it necessary?
// impl<E: Element> SuperElement<E> for E {
//     fn upcast(child: E) -> Self {
//         child
//     }

//     fn downcast<'a>(refm: Self::Mut<'a>) -> <E as Element>::Mut<'a> {
//         refm
//     }
// }

pub trait AnyView<State, Action, Element> {}

impl<State, Action, DynamicElement, V> AnyView<State, Action, DynamicElement> for V
where
    DynamicElement: SuperElement<V::Element>,
    V: View<State, Action>,
{
}

// Model version of Masonry

pub trait Widget: 'static + Any {
    fn as_mut_any(&mut self) -> &mut dyn Any;
}
pub struct WidgetPod<W: Widget> {
    widget: W,
}
pub struct WidgetMut<'a, W: Widget> {
    value: &'a mut W,
}
impl Widget for Box<dyn Widget> {
    fn as_mut_any(&mut self) -> &mut dyn Any {
        self
    }
}

// Model version of xilem_masonry (`xilem`)

// Hmm, this implementation can't exist in `xilem` if `xilem_core` is a different crate
// due to the orphan rules...
impl<W: Widget> Element for WidgetPod<W> {
    type Mut<'a> = WidgetMut<'a, W>;
}

impl View<(), ()> for Button {
    type ViewState = ();
    type Element = WidgetPod<ButtonWidget>;
}

pub struct Button {}

pub struct ButtonWidget {}
impl Widget for ButtonWidget {
    fn as_mut_any(&mut self) -> &mut dyn Any {
        self
    }
}

impl<W: Widget> SuperElement<WidgetPod<W>> for WidgetPod<Box<dyn Widget>> {
    fn upcast(child: WidgetPod<W>) -> Self {
        WidgetPod {
            widget: Box::new(child.widget),
        }
    }

    fn downcast<'a>(refm: Self::Mut<'a>) -> <WidgetPod<W> as Element>::Mut<'a> {
        WidgetMut {
            value: refm.value.as_mut_any().downcast_mut().unwrap(),
        }
    }
}
