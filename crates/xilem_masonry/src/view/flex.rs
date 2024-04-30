use std::marker::PhantomData;

use masonry::{
    widget::{self, Axis, CrossAxisAlignment, FlexParams, WidgetMut},
    Widget, WidgetPod,
};

use crate::{
    sequence::SequenceCompatible, ChangeFlags, ElementSplice, VecSplice, View, ViewElement,
    ViewSequence,
};

// TODO: Allow configuring flex properties. I think this actually needs its own view trait?
pub fn flex<VT, Marker>(sequence: VT) -> Flex<VT, Marker> {
    Flex {
        phantom: PhantomData,
        sequence,
        axis: Axis::Vertical,
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
                FlexElement::FixedChild(widget, alignment) => {
                    view.with_flex_child_pod(widget, FlexParams::new(None, alignment))
                }
                FlexElement::FlexChild(widget, alignment, flex) => {
                    view.with_flex_child_pod(widget, FlexParams::new(flex, alignment))
                }
                FlexElement::DefaultSpacer => view.with_default_spacer(),
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
    /// The default spacer
    ///
    /// TODO: This has trouble when the axis changes
    DefaultSpacer,
    FlexSpacer(f64),
    FixedSpacer(f64),
    FixedChild(WidgetPod<Box<dyn Widget>>, Option<CrossAxisAlignment>),
    FlexChild(WidgetPod<Box<dyn Widget>>, Option<CrossAxisAlignment>, f64),
}

#[derive(Copy, Clone, PartialEq)]
pub enum FlexSpacer {
    Fixed(f64),
    Flex(f64),
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

    fn downcast<'m>(erased: <Self::Erased as ViewElement>::Mut<'m>) -> Self::Mut<'m> {
        erased
    }

    fn reborrow<'r, 'm>(reference_mut: &'r mut Self::Mut<'m>) -> Self::Mut<'r> {
        FlexElementMut {
            parent: reference_mut.parent.reborrow(),
            ix: reference_mut.ix,
        }
    }
}

impl SequenceCompatible<WidgetPod<Box<dyn Widget>>> for FlexElement {
    fn into_item(element: WidgetPod<Box<dyn Widget>>) -> Self {
        Self::FixedChild(element, None)
    }

    fn access_mut<'a, R>(
        mut reference: Self::Mut<'a>,
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
            FlexElement::DefaultSpacer => self.element.insert_default_spacer(self.ix),
            FlexElement::FixedChild(widget, _axis) => {
                self.element.insert_child_pod(self.ix, widget);
            }
            FlexElement::FlexChild(widget, _axis, _len) => {
                self.element.insert_child_pod(self.ix, widget);
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
