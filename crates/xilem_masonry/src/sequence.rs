use std::num::NonZeroU64;

use masonry::{widget::WidgetMut, Widget, WidgetPod};

use crate::{ChangeFlags, MasonryView, MessageResult, ViewCx, ViewId};

#[allow(clippy::len_without_is_empty)]
pub trait ElementSplice {
    /// Insert a new element at the current index in the resulting collection (and increment the index by 1)
    fn push(&mut self, element: WidgetPod<Box<dyn Widget>>);
    /// Mutate the next existing element, and add it to the resulting collection (and increment the index by 1)
    // TODO: This should actually return `WidgetMut<dyn Widget>`, but that isn't supported in Masonry itself yet
    fn mutate(&mut self) -> WidgetMut<Box<dyn Widget>>;
    /// Delete the next n existing elements (this doesn't change the index)
    fn delete(&mut self, n: usize);
    /// Current length of the elements collection
    // TODO: Is `len` needed?
    fn len(&self) -> usize;
}

/// This trait represents a (possibly empty) sequence of views.
///
/// It is up to the parent view how to lay out and display them.
pub trait ViewSequence<State, Action, Marker>: Send + 'static {
    /// Build the associated widgets and initialize all states.
    ///
    /// To be able to monitor changes (e.g. tree-structure tracking) rather than just adding elements,
    /// this takes an element splice as well (when it could be just a `Vec` otherwise)
    fn build(&self, cx: &mut ViewCx, elements: &mut dyn ElementSplice);

    /// Update the associated widget.
    ///
    /// Returns `true` when anything has changed.
    fn rebuild(
        &self,
        cx: &mut ViewCx,
        prev: &Self,
        elements: &mut dyn ElementSplice,
    ) -> ChangeFlags;

    /// Propagate a message.
    ///
    /// Handle a message, propagating to elements if needed. Here, `id_path` is a slice
    /// of ids beginning at an element of this view_sequence.
    fn message(
        &self,
        id_path: &[ViewId],
        message: Box<dyn std::any::Any>,
        app_state: &mut State,
    ) -> MessageResult<Action>;

    /// Returns the current amount of widgets built by this sequence.
    fn count(&self) -> usize;
}

/// Workaround for trait ambiguity
///
/// These need to be public for type inference
#[doc(hidden)]
pub struct WasAView;
#[doc(hidden)]
/// See [`WasAView`]
pub struct WasASequence;

impl<State, Action, View: MasonryView<State, Action>> ViewSequence<State, Action, WasAView>
    for View
{
    fn build(&self, cx: &mut ViewCx, elements: &mut dyn ElementSplice) {
        let element = self.build(cx);
        elements.push(element.boxed());
    }

    fn rebuild(
        &self,
        cx: &mut ViewCx,
        prev: &Self,
        elements: &mut dyn ElementSplice,
    ) -> ChangeFlags {
        let mut element = elements.mutate();
        let downcast = element.downcast::<View::Element>();

        if let Some(element) = downcast {
            self.rebuild(cx, prev, element)
        } else {
            unreachable!("Tree structure tracking got wrong element type")
        }
    }

    fn message(
        &self,
        id_path: &[ViewId],
        message: Box<dyn std::any::Any>,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        self.message(id_path, message, app_state)
    }

    fn count(&self) -> usize {
        1
    }
}

impl<State, Action, Marker, VT: ViewSequence<State, Action, Marker>>
    ViewSequence<State, Action, (WasASequence, Marker)> for Option<VT>
{
    fn build(&self, cx: &mut ViewCx, elements: &mut dyn ElementSplice) {
        match self {
            Some(this) => {
                this.build(cx, elements);
            }
            None => (),
        }
    }

    fn rebuild(
        &self,
        cx: &mut ViewCx,
        prev: &Self,
        elements: &mut dyn ElementSplice,
    ) -> ChangeFlags {
        match (self, prev) {
            (Some(this), Some(prev)) => this.rebuild(cx, prev, elements),
            (None, Some(prev)) => {
                let count = prev.count();
                elements.delete(count);

                ChangeFlags::CHANGED
            }
            (Some(this), None) => {
                this.build(cx, elements);
                ChangeFlags::CHANGED
            }
            (None, None) => ChangeFlags::UNCHANGED,
        }
    }

    fn message(
        &self,
        id_path: &[ViewId],
        message: Box<dyn std::any::Any>,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        if let Some(this) = self {
            this.message(id_path, message, app_state)
        } else {
            MessageResult::Stale(message)
        }
    }

    fn count(&self) -> usize {
        match self {
            Some(this) => this.count(),
            None => 0,
        }
    }
}

// TODO: We use raw indexing for this value. What would make it invalid?
impl<T, A, Marker, VT: ViewSequence<T, A, Marker>> ViewSequence<T, A, (WasASequence, Marker)>
    for Vec<VT>
{
    fn build(&self, cx: &mut ViewCx, elements: &mut dyn ElementSplice) {
        self.iter().enumerate().for_each(|(i, child)| {
            let i: u64 = i.try_into().unwrap();
            let id = NonZeroU64::new(i + 1).unwrap();
            cx.with_id(ViewId::for_type::<VT>(id), |cx| child.build(cx, elements));
        });
    }

    fn rebuild(
        &self,
        cx: &mut ViewCx,
        prev: &Self,
        elements: &mut dyn ElementSplice,
    ) -> ChangeFlags {
        let mut changed = ChangeFlags::UNCHANGED;
        for (i, (child, child_prev)) in self.iter().zip(prev).enumerate() {
            let i: u64 = i.try_into().unwrap();
            let id = NonZeroU64::new(i + 1).unwrap();
            cx.with_id(ViewId::for_type::<VT>(id), |cx| {
                let el_changed = child.rebuild(cx, child_prev, elements);
                changed.changed |= el_changed.changed;
            });
        }
        let n = self.len();
        if n < prev.len() {
            let n_delete = prev[n..].iter().map(ViewSequence::count).sum();
            elements.delete(n_delete);
            changed.changed |= ChangeFlags::CHANGED.changed;
        } else if n > prev.len() {
            // This suggestion from clippy is kind of bad, because we use the absolute index in the id
            #[allow(clippy::needless_range_loop)]
            for ix in prev.len()..n {
                let id_u64: u64 = ix.try_into().unwrap();
                let id = NonZeroU64::new(id_u64 + 1).unwrap();
                cx.with_id(ViewId::for_type::<VT>(id), |cx| {
                    self[ix].build(cx, elements);
                });
            }
            changed.changed |= ChangeFlags::CHANGED.changed;
        }
        changed
    }

    fn message(
        &self,
        id_path: &[ViewId],
        message: Box<dyn std::any::Any>,
        app_state: &mut T,
    ) -> MessageResult<A> {
        let (start, rest) = id_path
            .split_first()
            .expect("Id path has elements for vector");
        let index_plus_one: usize = start.routing_id().get().try_into().unwrap();
        self[index_plus_one - 1].message(rest, message, app_state)
    }

    fn count(&self) -> usize {
        self.iter().map(ViewSequence::count).sum()
    }
}
