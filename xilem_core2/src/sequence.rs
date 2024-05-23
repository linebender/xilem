// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Support for sequences of views with a shared element type.

use crate::{DynMessage, MessageResult, SuperElement, View, ViewElement, ViewId, ViewPathTracker};

/// A sequence of views.
///
/// It is up to the parent view how to lay out and display them.
pub trait ViewSequence<State, Action, Context: ViewPathTracker, Element: ViewElement, Marker>:
    'static
{
    type SeqState;

    /// Build the associated widgets into `elements` and initialize all states.
    #[must_use]
    fn seq_build(
        &self,
        ctx: &mut Context,
        elements: &mut impl ElementSplice<Element>,
    ) -> Self::SeqState;

    /// Update the associated widgets.
    fn seq_rebuild(
        &self,
        prev: &Self,
        seq_state: &mut Self::SeqState,
        ctx: &mut Context,
        elements: &mut impl ElementSplice<Element>,
    );

    /// Update the associated widgets.
    fn seq_teardown(
        &self,
        seq_state: &mut Self::SeqState,
        ctx: &mut Context,
        elements: &mut impl ElementSplice<Element>,
    );

    /// Propagate a message.
    ///
    /// Handle a message, propagating to elements if needed. Here, `id_path` is a slice
    /// of ids beginning at an element of this view_sequence.
    fn seq_message(
        &self,
        seq_state: &mut Self::SeqState,
        id_path: &[ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> MessageResult<Action>;
}

/// A temporary "splice" to add, update and delete in an (ordered) sequence of elements.
/// It is mainly intended for view sequences.
pub trait ElementSplice<Element: ViewElement> {
    /// Insert a new element at the current index in the resulting collection.
    fn push(&mut self, element: Element);
    /// Mutate the next existing element.
    fn mutate<R>(&mut self, f: impl FnOnce(Element::Mut<'_>) -> R) -> R;
    /// Don't make any changes to the next n existing elements.
    fn skip(&mut self, n: usize);
    /// Delete the next existing element, after running a function on it.
    fn delete<R>(&mut self, f: impl FnOnce(Element::Mut<'_>) -> R) -> R;
}

/// Workaround for trait ambiguity.
///
/// These need to be public for type inference.
#[doc(hidden)]
pub struct WasAView;
/// See [`WasAView`].
#[doc(hidden)]
pub struct WasASequence;

impl<State, Action, Context, V, Element> ViewSequence<State, Action, Context, Element, WasAView>
    for V
where
    Context: ViewPathTracker,
    V: View<State, Action, Context>,
    Element: SuperElement<V::Element>,
    V::Element: ViewElement,
{
    type SeqState = V::ViewState;

    fn seq_build(
        &self,
        ctx: &mut Context,
        elements: &mut impl ElementSplice<Element>,
    ) -> Self::SeqState {
        let (element, view_state) = self.build(ctx);
        elements.push(Element::upcast(element));
        view_state
    }
    fn seq_rebuild(
        &self,
        prev: &Self,
        seq_state: &mut Self::SeqState,
        ctx: &mut Context,
        elements: &mut impl ElementSplice<Element>,
    ) {
        // Mutate the item we added in `seq_build`
        elements.mutate(|this_element| {
            Element::with_downcast(this_element, |element| {
                self.rebuild(prev, seq_state, ctx, element);
            });
        });
    }
    fn seq_teardown(
        &self,
        seq_state: &mut Self::SeqState,
        ctx: &mut Context,
        elements: &mut impl ElementSplice<Element>,
    ) {
        elements.delete(|this_element| {
            Element::with_downcast(this_element, |element| {
                self.teardown(seq_state, ctx, element);
            });
        });
    }

    fn seq_message(
        &self,
        seq_state: &mut Self::SeqState,
        id_path: &[ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        self.message(seq_state, id_path, message, app_state)
    }
}

/// The state used to implement `ViewSequence` for `Option<impl ViewSequence>`
pub struct OptionSeqState<InnerState> {
    /// The current state.
    ///
    /// Will be `None` if the previous value was `None`.
    inner: Option<InnerState>,
    /// The generation this option is at.
    ///
    /// If the inner sequence was Some, then None, then Some, the sequence
    /// is treated as a new sequence, as e.g. build has been called again.
    generation: u64,
}

impl<State, Action, Context, Element, Marker, Seq>
    ViewSequence<State, Action, Context, Element, (Marker, WasASequence)> for Option<Seq>
where
    Seq: ViewSequence<State, Action, Context, Element, Marker>,
    Context: ViewPathTracker,
    Element: ViewElement,
{
    type SeqState = OptionSeqState<Seq::SeqState>;

    fn seq_build(
        &self,
        ctx: &mut Context,
        elements: &mut impl ElementSplice<Element>,
    ) -> Self::SeqState {
        let generation = 0;
        match self {
            Some(seq) => {
                let inner =
                    ctx.with_id(ViewId::new(generation), |ctx| seq.seq_build(ctx, elements));
                OptionSeqState {
                    inner: Some(inner),
                    generation,
                }
            }
            None => OptionSeqState {
                inner: None,
                generation,
            },
        }
    }

    fn seq_rebuild(
        &self,
        prev: &Self,
        seq_state: &mut Self::SeqState,
        ctx: &mut Context,
        elements: &mut impl ElementSplice<Element>,
    ) {
        // If `prev` was `Some`, we set `seq_state` in reacting to it (and building the inner view)
        // This could only fail if some malicious parent view was messing with our internal state
        // (i.e. mixing up the state from different instances)
        assert_eq!(prev.is_some(), seq_state.inner.is_some());
        match (self, prev.as_ref().zip(seq_state.inner.as_mut())) {
            (Some(seq), Some((prev, inner_state))) => {
                // Perform a normal rebuild
                ctx.with_id(ViewId::new(seq_state.generation), |ctx| {
                    seq.seq_rebuild(prev, inner_state, ctx, elements)
                });
            }
            (None, None) => {
                // Nothing to do, there is no corresponding element
            }
            (None, Some(_)) => {
                // The sequence is newly re-added
            }
            (Some(_), None) => {
                // The sequence has just been destroyed, teardown the old view
                // We increment the generation only on the falling edge, as the (None, None) case
                // doesn't handle the
                seq_state.generation += 1;
            }
        }
    }

    fn seq_teardown(
        &self,
        seq_state: &mut Self::SeqState,
        ctx: &mut Context,
        elements: &mut impl ElementSplice<Element>,
    ) {
        todo!()
    }

    fn seq_message(
        &self,
        seq_state: &mut Self::SeqState,
        id_path: &[ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        todo!()
    }
}
