use std::marker::PhantomData;

use masonry::{
    widget::{self, Axis, FlexParams, WidgetMut},
    Widget, WidgetPod,
};

use crate::{
    sequence::SequenceCompatible, ChangeFlags, ElementSplice, MasonryView, VecSplice, View,
    ViewElement, ViewSequence,
};

// TODO: Allow configuring flex properties. I think this actually needs its own view trait?
pub fn flex<VT, Marker>(sequence: VT) -> Flex<VT, Marker> {
    Flex {
        phantom: PhantomData,
        sequence,
        axis: Axis::Vertical,
    }
}

pub fn flex_item<V>(view: V, params: impl Into<FlexParams>) -> FlexWrapper<V> {
    FlexWrapper {
        params: params.into(),
        view,
    }
}

pub struct Flex<VT, Marker> {
    sequence: VT,
    axis: Axis,
    phantom: PhantomData<fn() -> Marker>,
}

impl<VT, Marker> Flex<VT, Marker> {
    pub fn direction(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }
}

impl<State, Action, Marker: 'static, Seq> View<State, Action> for Flex<Seq, Marker>
where
    Seq: ViewSequence<State, Action, Marker, FlexElement>,
{
    type Element = WidgetPod<widget::Flex>;
    type ViewState = Seq::SeqState;

    fn build(&self, cx: &mut crate::ViewCx) -> (masonry::WidgetPod<widget::Flex>, Self::ViewState) {
        let mut elements = Vec::new();
        let mut scratch = Vec::new();
        let mut splice = VecSplice::new(&mut elements, &mut scratch);
        let seq_state = self.sequence.build(cx, &mut splice);
        let mut view = widget::Flex::for_axis(self.axis);
        debug_assert!(
            scratch.is_empty(),
            // TODO: Not at all confident about this, but linear_layout makes this assumption
            "ViewSequence shouldn't leave splice in strange state"
        );
        for item in elements.drain(..) {
            view = match item {
                FlexElement::FlexSpacer(flex) => view.with_flex_spacer(flex),
                FlexElement::FixedSpacer(len) => view.with_spacer(len),
                FlexElement::Child(widget, params) => view.with_flex_child_pod(*widget, params),
            }
        }
        (WidgetPod::new(view), seq_state)
    }

    fn rebuild(
        &self,
        view_state: &mut Self::ViewState,
        cx: &mut crate::ViewCx,
        prev: &Self,
        mut element: widget::WidgetMut<widget::Flex>,
    ) -> ChangeFlags {
        let mut changeflags = ChangeFlags::UNCHANGED;
        if prev.axis != self.axis {
            element.set_direction(self.axis);
            changeflags.changed |= ChangeFlags::CHANGED.changed;
        }
        let mut splice = FlexSplice { ix: 0, element };
        changeflags.changed |= self
            .sequence
            .rebuild(view_state, cx, &prev.sequence, &mut splice)
            .changed;
        changeflags
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[crate::ViewId],
        message: Box<dyn std::any::Any>,
        app_state: &mut State,
    ) -> crate::MessageResult<Action> {
        self.sequence
            .message(view_state, id_path, message, app_state)
    }
}

pub enum FlexElement {
    // /// The default spacer
    // ///
    // /// TODO: This has trouble when the axis changes
    // DefaultSpacer,
    FlexSpacer(f64),
    FixedSpacer(f64),
    // Avoid making the enum massive for the spacer cases by boxing
    Child(Box<WidgetPod<Box<dyn Widget>>>, FlexParams),
}

#[derive(Copy, Clone, PartialEq)]
pub enum FlexSpacer {
    Fixed(f64),
    Flex(f64),
}

pub struct FlexWrapper<V> {
    view: V,
    params: FlexParams,
}

impl<State, Action> View<State, Action> for FlexSpacer {
    type Element = FlexElement;

    type ViewState = ();

    fn build(&self, _: &mut crate::ViewCx) -> (Self::Element, Self::ViewState) {
        let el = match self {
            FlexSpacer::Fixed(len) => FlexElement::FixedSpacer(*len),
            FlexSpacer::Flex(flex) => FlexElement::FlexSpacer(*flex),
        };
        (el, ())
    }

    fn rebuild(
        &self,
        (): &mut Self::ViewState,
        _: &mut crate::ViewCx,
        prev: &Self,
        mut element: <Self::Element as ViewElement>::Mut<'_>,
    ) -> ChangeFlags {
        if self != prev {
            match self {
                FlexSpacer::Fixed(len) => element.parent.update_spacer_fixed(element.ix, *len),
                FlexSpacer::Flex(flex) => element.parent.update_spacer_flex(element.ix, *flex),
            }
            ChangeFlags::UNCHANGED
        } else {
            ChangeFlags::UNCHANGED
        }
    }

    fn message(
        &self,
        _: &mut Self::ViewState,
        _: &[crate::ViewId],
        _: Box<dyn std::any::Any>,
        _: &mut State,
    ) -> crate::MessageResult<Action> {
        unreachable!()
    }
}

impl<State, Action, V: MasonryView<State, Action>> View<State, Action> for FlexWrapper<V> {
    type Element = FlexElement;

    type ViewState = V::ViewState;

    fn build(&self, cx: &mut crate::ViewCx) -> (Self::Element, Self::ViewState) {
        let (inner, state) = self.view.build(cx);
        (
            FlexElement::Child(Box::new(inner.boxed()), self.params),
            state,
        )
    }

    fn rebuild(
        &self,
        view_state: &mut Self::ViewState,
        cx: &mut crate::ViewCx,
        prev: &Self,
        mut element: <Self::Element as ViewElement>::Mut<'_>,
    ) -> ChangeFlags {
        if self.params != prev.params {
            element
                .parent
                .update_child_flex_params(element.ix, self.params);
        }
        let mut element = element
            .parent
            .child_mut(element.ix)
            .expect("FlexWrapper always has a widget child");
        self.view.rebuild(
            view_state,
            cx,
            &prev.view,
            element
                .downcast()
                .expect("Element should have correct element type"),
        )
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[crate::ViewId],
        message: Box<dyn std::any::Any>,
        app_state: &mut State,
    ) -> crate::MessageResult<Action> {
        self.view.message(view_state, id_path, message, app_state)
    }
}

pub struct FlexElementMut<'a> {
    parent: WidgetMut<'a, widget::Flex>,
    ix: usize,
}

impl ViewElement for FlexElement {
    type Mut<'a> = FlexElementMut<'a>;

    type Erased = Self;

    fn erase(self) -> Self::Erased {
        self
    }

    fn downcast(erased: <Self::Erased as ViewElement>::Mut<'_>) -> Self::Mut<'_> {
        erased
    }

    fn reborrow<'r>(reference_mut: &'r mut Self::Mut<'_>) -> Self::Mut<'r> {
        FlexElementMut {
            parent: reference_mut.parent.reborrow(),
            ix: reference_mut.ix,
        }
    }
}

impl SequenceCompatible<WidgetPod<Box<dyn Widget>>> for FlexElement {
    fn into_item(element: WidgetPod<Box<dyn Widget>>) -> Self {
        Self::Child(Box::new(element), FlexParams::new(None, None))
    }

    fn access_mut<R>(
        mut reference: Self::Mut<'_>,
        f: impl FnOnce(&mut <WidgetPod<Box<dyn Widget>> as ViewElement>::Mut<'_>) -> R,
    ) -> R {
        let mut child = reference
            .parent
            .child_mut(reference.ix)
            .expect("be a child mut as precondition of this function");
        f(&mut child)
    }
}

struct FlexSplice<'w> {
    ix: usize,
    element: WidgetMut<'w, widget::Flex>,
}

impl ElementSplice<FlexElement> for FlexSplice<'_> {
    fn push(&mut self, element: FlexElement) {
        match element {
            FlexElement::FlexSpacer(flex) => self.element.insert_flex_spacer(self.ix, flex),
            FlexElement::FixedSpacer(len) => self.element.insert_spacer(self.ix, len),
            FlexElement::Child(widget, params) => {
                self.element.insert_flex_child_pod(self.ix, *widget, params);
            }
        }
        self.ix += 1;
    }

    fn mutate(&mut self) -> FlexElementMut {
        let res = FlexElementMut {
            parent: self.element.reborrow(),
            ix: self.ix,
        };
        self.ix += 1;
        res
    }

    fn delete(&mut self, n: usize) {
        let mut deleted_count = 0;
        while deleted_count < n {
            self.element.remove_child(self.ix);
            deleted_count += 1;
        }
    }
}
