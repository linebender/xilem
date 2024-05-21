// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0
#![no_std]
// TODO: Point at documentation for this pattern of README include
#![doc = concat!(
" 
<!-- This license link is in a .rustdoc-hidden section, but we may as well give the correct link -->
[LICENSE]: https://github.com/linebender/xilem/blob/main/xilem_core/LICENSE
<!-- intra-doc-links go here -->
<style>
.rustdoc-hidden { display: none; }
</style>

<!-- Hide the header section of the README when using rustdoc -->
<div style=\"display:none\">
",
    include_str!("../README.md"),
)]

extern crate alloc;

use alloc::boxed::Box;

mod element;

use core::any::Any;

pub use element::{Element, SuperElement};

// Needed:
// 1) View trait
// 2) ViewSequence trait
// 3) AnyView trait
//
// View trait has an element type
// AnyView trait is implemented for any sequence which can
// ViewSequence trait

/* /// Types which can route a message view to a child [`View`].
// TODO: This trait needs to be different for desktop hot reloading
pub trait ViewMessage<State, Action> {}
 */

/// A lightweight, short-lived representation of the state of a retained
/// structure, usually a user interface node.
///
/// This is the central reactivity primitive in Xilem.
/// An app will generate a tree of these objects (the view tree) to represent
/// the state it wants to show in its element tree.
/// The framework will then run methods on these views to create the associated
/// element tree, or to perform incremental updates to the element tree.
/// Once this process is complete, the element tree will reflect the view tree.
/// The view tree is also used to dispatch messages, such as those sent when a
/// user presses a button.
///
/// The view tree is transitory and is retained only long enough to dispatch
/// messages and then serve as a reference for diffing for the next view tree.
///
/// The `View` trait is parameterized by `State`, which is known as the "app state",
/// and also a type for actions which are passed up the tree in message
/// propagation. During message handling, mutable access to the app state is
/// given to view nodes, which will in turn often expose it to callbacks.
// TODO: What is the `Action` type actually used for
pub trait View<State, Action = ()> {
    /// The element type which this view operates on.
    type Element: Element;
    /// The state needed for this view to route messages to
    /// the correct child view.
    type ViewState;
}
// TODO: What do we want to do here? This impl seems nice, but is it necessary?
// It lets you trivially have sequences of types with a heterogenous element type,
// but how common are those in practice?
// It conflicts with the xilem_masonry dynamic implementation (assuming that `Box<dyn Widget>: Widget` holds)
// impl<E: Element> SuperElement<E> for E {
//     fn upcast(child: E) -> Self { child }
//     fn downcast<'a>(refm: Self::Mut<'a>) -> <E as Element>::Mut<'a> { refm }
// }

/// A view which can have any view type where the [`View::Element`] is compatible with
/// `Element`.
///
/// This is primarily used for type erasure of views.
/// This is useful for a view which can be either of two view types, in addition to
// TODO: Mention `Either` when we have implemented that?
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

    fn with_reborrow_val<'o, R: 'static>(
        this: Self::Mut<'o>,
        f: impl FnOnce(Self::Mut<'_>) -> R,
    ) -> (Self::Mut<'o>, R) {
        let value = WidgetMut { value: this.value };
        let ret = f(value);
        (this, ret)
    }
}

impl View<(), ()> for Button {
    type Element = WidgetPod<ButtonWidget>;
    type ViewState = ();
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
    fn with_downcast_val<'a, R>(
        this: Self::Mut<'a>,
        f: impl FnOnce(<WidgetPod<W> as Element>::Mut<'_>) -> R,
    ) -> (Self::Mut<'a>, R) {
        let value = WidgetMut {
            value: this.value.as_mut_any().downcast_mut().expect(
                "this widget should have been created from a child widget of type `W` in `Self::upcast`",
            ),
        };
        let ret = f(value);
        (this, ret)
    }
}
