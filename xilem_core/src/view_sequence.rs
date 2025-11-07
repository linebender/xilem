// Copyright 2023 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Support for sequences of views with a shared element type.

use crate::{
    AppendVec, Arg, ElementSplice, MessageContext, MessageResult, SuperElement, View, ViewArgument,
    ViewElement, ViewMarker, ViewPathTracker,
};

/// Classes that a [`ViewSequence`] can be a member of, grouped based on the number
/// of elements it is known to contain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Count {
    /// This sequence is known to have no elements.
    Zero,
    /// This sequence is known to have exactly one element.
    One,
    // TODO: AtMostOne - useful for Option (and e.g. `SizedBox`?).
    /// This sequence may have any number of elements.
    Many,
    /// The number of elements this sequence has is not statically known.
    ///
    /// This is used for [`AnyView`](crate::AnyView).
    Unknown,
}

impl Count {
    /// Combine the counts of multiple children.
    pub const fn combine<const N: usize>(vals: [Self; N]) -> Self {
        #![expect(clippy::use_self, reason = "Easier to read in this case as `Count`.")]
        let mut idx = 0;
        let mut current_count = Count::Zero;
        while idx < N {
            idx += 1;
            match vals[idx] {
                Count::Zero => {}
                Count::One if matches!(current_count, Count::Zero) => {
                    current_count = Count::One;
                }
                Count::One if matches!(current_count, Count::One) => {
                    current_count = Count::Many;
                }
                Count::One => {}
                // Many overwrites everything, including unknown
                // Because if we have one "many" child, we definitely have several
                Count::Many => {
                    current_count = Count::Many;
                }
                Count::Unknown if !matches!(current_count, Count::Many) => {
                    current_count = Count::Unknown;
                }
                _ => panic!("How to report this properly"),
            }
        }
        current_count
    }

    /// The resulting count if there are (potentially) multiple of this sequence.
    pub const fn multiple(self) -> Self {
        Self::combine([self, self])
    }
}

// --- MARK: Trait

/// Views for ordered sequences of elements.
///
/// Generally, a container view will internally contain a `ViewSequence`.
/// The child elements of the container will be updated by the `ViewSequence`.
///
/// This is implemented for:
///  - Any single [`View`], where the view's element type
///    is [compatible](SuperElement) with the sequence's element type.
///    This is the root implementation, by which the sequence actually
///    updates the relevant element.
///  - An `Option` of a `ViewSequence` value.
///    The elements of the inner sequence will be inserted into the
///    sequence if the value is `Some`, and removed once the value is `None`.
///  - A [`Vec`] of `ViewSequence` values.
///    Note that this will have persistent allocation with size proportional
///    to the *longest* `Vec` which is ever provided in the View tree, as this
///    uses a generational indexing scheme.
///  - An [`array`] of `ViewSequence` values.
///  - Tuples of `ViewSequences` with up to 15 elements.
///    These can be nested if an ad-hoc sequence of more than 15 sequences is needed.
pub trait ViewSequence<State, Action, Context, Element>: 'static
where
    State: ViewArgument,
    Context: ViewPathTracker,
    Element: ViewElement,
{
    /// The associated state of this sequence. The main purposes of this are:
    /// - To store generations and other data needed to avoid routing stale messages
    ///   to incorrect views.
    /// - To pass on the state of child sequences, or a child View's [`ViewState`].
    ///
    /// The type used for this associated type cannot be treated as public API; this is
    /// internal state to the `ViewSequence` implementation.
    /// That is, `ViewSequence` implementations are permitted to change the type they use for this
    ///  during even a patch release of their crate.
    ///
    /// [`ViewState`]: View::ViewState
    type SeqState;

    /// The class of sequence this is, grouped based on how many elements it may contain.
    /// This is useful for making bounds based on the expected number of child elements.
    const ELEMENTS_COUNT: Count;

    /// Build the associated widgets into `elements` and initialize all states.
    #[must_use]
    fn seq_build(
        &self,
        ctx: &mut Context,
        elements: &mut AppendVec<Element>,
        app_state: Arg<'_, State>,
    ) -> Self::SeqState;

    /// Update the associated widgets.
    fn seq_rebuild(
        &self,
        prev: &Self,
        seq_state: &mut Self::SeqState,
        ctx: &mut Context,
        elements: &mut impl ElementSplice<Element>,
        app_state: Arg<'_, State>,
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
    /// Handle a message, propagating to child views if needed.
    /// The context contains both the path of this view, and the remaining
    /// path that the rest of the implementation needs to go along.
    /// The first items in the remaining part of thtis path will be those added
    /// in build and/or rebuild.
    ///
    /// The provided `elements` must be at the same [`index`](ElementSplice::index)
    /// it was at when `rebuild` (or `build`) was last called on this sequence.
    ///
    /// The easiest way to achieve this is to cache the index reached before any child
    /// sequence's build/rebuild, and skip to that value.
    /// Note that the amount you will need to skip to reach this value won't be the index
    /// directly, but instead must be the difference between this index and the value of
    /// the index at the start of your build/rebuild.
    // Potential optimisation: Sequence implementations can be grouped into three classes:
    // 1) Statically known size (e.g. a single element, a tuple with only statically known size)
    // 2) Linear time known size (e.g. a tuple of linear or better known size, a vec of statically known size, an option)
    // 3) Dynamically known size (e.g. a vec)
    // For case 1 and maybe case 2, we don't need to store the indices, and could instead rebuild them dynamically.
    fn seq_message(
        &self,
        seq_state: &mut Self::SeqState,
        message: &mut MessageContext,
        elements: &mut impl ElementSplice<Element>,
        app_state: Arg<'_, State>,
    ) -> MessageResult<Action>;
}

// --- MARK: For V: View

impl<State, Action, Context, V, Element> ViewSequence<State, Action, Context, Element> for V
where
    State: ViewArgument,
    Context: ViewPathTracker,
    V: View<State, Action, Context> + ViewMarker,
    Element: SuperElement<V::Element, Context>,
    V::Element: ViewElement,
{
    type SeqState = V::ViewState;

    const ELEMENTS_COUNT: Count = Count::One;

    fn seq_build(
        &self,
        ctx: &mut Context,
        elements: &mut AppendVec<Element>,
        app_state: Arg<'_, State>,
    ) -> Self::SeqState {
        let (element, view_state) = self.build(ctx, app_state);
        elements.push(Element::upcast(ctx, element));
        view_state
    }
    fn seq_rebuild(
        &self,
        prev: &Self,
        seq_state: &mut Self::SeqState,
        ctx: &mut Context,
        elements: &mut impl ElementSplice<Element>,
        app_state: Arg<'_, State>,
    ) {
        // Mutate the item we added in `seq_build`
        elements.mutate(|this_element| {
            Element::with_downcast(this_element, |element| {
                self.rebuild(prev, seq_state, ctx, element, app_state);
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
        message: &mut MessageContext,
        elements: &mut impl ElementSplice<Element>,
        app_state: Arg<'_, State>,
    ) -> MessageResult<Action> {
        elements.mutate(|this_element| {
            Element::with_downcast_val(this_element, |element| {
                self.message(seq_state, message, element, app_state)
            })
            .1
        })
    }
}
