use crate::{
    AppendVec, DynMessage, ElementSplice, MessageResult, Mut, View, ViewElement, ViewId,
    ViewPathTracker, ViewSequence,
};

/// Statically typed alternative to the type-erased `AnyView`
/// This view container can switch between two different views.
/// It can also be used for alternating between different `ViewSequence`s
///
/// # Examples
///
/// Basic usage:
///
/// ```ignore
/// // As view
/// let mut v = OneOf2::A(my_view());
/// v = OneOf2::B(my_other_view());
/// // As view sequence
/// let mut seq = OneOf2::A((my_view(), my_other_view()));
/// seq = OneOf2::B(vec![my_view()]);
/// ```
#[allow(missing_docs)]
pub enum OneOf2<A, B> {
    A(A),
    B(B),
}

impl<T, A: AsRef<T>, B: AsRef<T>> AsRef<T> for OneOf2<A, B> {
    fn as_ref(&self) -> &T {
        match self {
            OneOf2::A(e) => <A as AsRef<T>>::as_ref(e),
            OneOf2::B(e) => <B as AsRef<T>>::as_ref(e),
        }
    }
}

/// To be able to use this `OneOf2` as a `View`, it's necessary to implement `OneOf2Ctx` for your `ViewCtx` type
pub trait OneOf2Ctx<A: ViewElement, B: ViewElement> {
    /// Element wrapper, that holds the current view element variant
    type OneOfTwoElement: ViewElement;

    /// Casts the view element `elem` to the `OneOf2::A` variant.
    /// `f` needs to be invoked with that inner `ViewElement`
    fn with_downcast_a(elem: &mut Mut<'_, Self::OneOfTwoElement>, f: impl FnMut(Mut<'_, A>));

    /// Casts the view element `elem` to the `OneOf2::B` variant.
    /// `f` needs to be invoked with that inner `ViewElement`
    fn with_downcast_b(elem: &mut Mut<'_, Self::OneOfTwoElement>, f: impl FnMut(Mut<'_, B>));

    /// Creates the wrapping element, this is used in `View::build` to wrap the inner view element variant
    fn upcast_one_of_two_element(elem: OneOf2<A, B>) -> Self::OneOfTwoElement;

    /// When the variant of the inner view element has changed, the wrapping element needs to be updated, this is used in `View::rebuild`
    fn update_one_of_two_element_mut(
        elem_mut: &mut Mut<'_, Self::OneOfTwoElement>,
        new_elem: OneOf2<A, B>,
    );
}

/// The state used to implement `View` or `ViewSequence` for `OneOf2`
#[doc(hidden)] // Implementation detail, public because of trait visibility rules
pub struct OneOf2State<A, B> {
    /// The current state of the inner view or view sequence.
    inner_state: OneOf2<A, B>,
    /// The generation this OneOf2 is at.
    ///
    /// If the variant of `OneOf2` has changed, i.e. the type of the inner view,
    /// the generation is incremented and used as ViewId in the id_path,
    /// to avoid (possibly async) messages reaching the wrong view,
    /// See the implementations of other `ViewSequence`s for more details
    generation: u64,
}

impl<A, B, Context, State, Action> View<State, Action, Context> for OneOf2<A, B>
where
    State: 'static,
    Action: 'static,
    Context: ViewPathTracker + OneOf2Ctx<A::Element, B::Element>,
    A: View<State, Action, Context>,
    B: View<State, Action, Context>,
{
    type Element = Context::OneOfTwoElement;

    type ViewState = OneOf2State<A::ViewState, B::ViewState>;

    fn build(&self, ctx: &mut Context) -> (Self::Element, Self::ViewState) {
        let generation = 0;
        let (element, inner_state) = ctx.with_id(ViewId::new(generation), |ctx| match self {
            OneOf2::A(e) => {
                let (element, state) = e.build(ctx);
                (
                    Context::upcast_one_of_two_element(OneOf2::A(element)),
                    OneOf2::A(state),
                )
            }
            OneOf2::B(e) => {
                let (element, state) = e.build(ctx);
                (
                    Context::upcast_one_of_two_element(OneOf2::B(element)),
                    OneOf2::B(state),
                )
            }
        });
        (
            element,
            OneOf2State {
                inner_state,
                generation,
            },
        )
    }

    fn rebuild<'e>(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut Context,
        mut element: Mut<'e, Self::Element>,
    ) -> Mut<'e, Self::Element> {
        // Type of the inner `View` stayed the same
        match (prev, self, &mut view_state.inner_state) {
            (OneOf2::A(prev), OneOf2::A(new), OneOf2::A(inner_state)) => {
                ctx.with_id(ViewId::new(view_state.generation), |ctx| {
                    Context::with_downcast_a(&mut element, |elem| {
                        new.rebuild(prev, inner_state, ctx, elem);
                    });
                });
                return element;
            }
            (OneOf2::B(prev), OneOf2::B(new), OneOf2::B(inner_state)) => {
                ctx.with_id(ViewId::new(view_state.generation), |ctx| {
                    Context::with_downcast_b(&mut element, |elem| {
                        new.rebuild(prev, inner_state, ctx, elem);
                    });
                });
                return element;
            }
            _ => (),
        };

        // View has changed type, teardown the old view
        // we can't use Self::teardown, because we still need access to the element

        ctx.with_id(ViewId::new(view_state.generation), |ctx| {
            match (prev, &mut view_state.inner_state) {
                (OneOf2::A(prev), OneOf2::A(old_state)) => {
                    Context::with_downcast_a(&mut element, |elem| {
                        prev.teardown(old_state, ctx, elem);
                    });
                }
                (OneOf2::B(prev), OneOf2::B(old_state)) => {
                    Context::with_downcast_b(&mut element, |elem| {
                        prev.teardown(old_state, ctx, elem);
                    });
                }
                _ => unreachable!(),
            };
        });

        // Overflow handling: u64 starts at 0, incremented by 1 always.
        // Can never realistically overflow, scale is too large.
        // If would overflow, wrap to zero. Would need async message sent
        // to view *exactly* `u64::MAX` versions of the view ago, which is implausible
        view_state.generation = view_state.generation.wrapping_add(1);

        // Create the new view

        ctx.with_id(ViewId::new(view_state.generation), |ctx| {
            match self {
                OneOf2::A(new) => {
                    let (new_element, state) = new.build(ctx);
                    view_state.inner_state = OneOf2::A(state);
                    Context::update_one_of_two_element_mut(&mut element, OneOf2::A(new_element));
                }
                OneOf2::B(new) => {
                    let (new_element, state) = new.build(ctx);
                    view_state.inner_state = OneOf2::B(state);
                    Context::update_one_of_two_element_mut(&mut element, OneOf2::B(new_element));
                }
            };
        });
        element
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut Context,
        mut element: Mut<'_, Self::Element>,
    ) {
        ctx.with_id(ViewId::new(view_state.generation), |ctx| {
            match (self, &mut view_state.inner_state) {
                (OneOf2::A(view), OneOf2::A(state)) => {
                    Context::with_downcast_a(&mut element, |elem| {
                        view.teardown(state, ctx, elem);
                    });
                }
                (OneOf2::B(view), OneOf2::B(state)) => {
                    Context::with_downcast_b(&mut element, |elem| {
                        view.teardown(state, ctx, elem);
                    });
                }
                _ => unreachable!(),
            }
        });
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        let (start, rest) = id_path
            .split_first()
            .expect("Id path has elements for OneOf2");
        if start.routing_id() != view_state.generation {
            // The message was sent to a previous edition of the inner value
            return MessageResult::Stale(message);
        }
        match (self, &mut view_state.inner_state) {
            (OneOf2::A(view), OneOf2::A(state)) => view.message(state, rest, message, app_state),
            (OneOf2::B(view), OneOf2::B(state)) => view.message(state, rest, message, app_state),
            _ => unreachable!(),
        }
    }
}

impl<State, Action, Context, Element, MarkerA, MarkerB, A, B>
    ViewSequence<State, Action, Context, Element, OneOf2<MarkerA, MarkerB>> for OneOf2<A, B>
where
    A: ViewSequence<State, Action, Context, Element, MarkerA>,
    B: ViewSequence<State, Action, Context, Element, MarkerB>,
    Context: ViewPathTracker,
    Element: ViewElement,
{
    type SeqState = OneOf2State<A::SeqState, B::SeqState>;

    fn seq_build(&self, ctx: &mut Context, elements: &mut AppendVec<Element>) -> Self::SeqState {
        let generation = 0;
        let inner_state = ctx.with_id(ViewId::new(generation), |ctx| match self {
            OneOf2::A(e) => OneOf2::A(e.seq_build(ctx, elements)),
            OneOf2::B(e) => OneOf2::B(e.seq_build(ctx, elements)),
        });
        OneOf2State {
            inner_state,
            generation,
        }
    }

    fn seq_rebuild(
        &self,
        prev: &Self,
        seq_state: &mut Self::SeqState,
        ctx: &mut Context,
        elements: &mut impl ElementSplice<Element>,
    ) {
        // Type of the inner `ViewSequence` stayed the same
        match (prev, self, &mut seq_state.inner_state) {
            (OneOf2::A(prev), OneOf2::A(new), OneOf2::A(inner_state)) => {
                ctx.with_id(ViewId::new(seq_state.generation), |ctx| {
                    new.seq_rebuild(prev, inner_state, ctx, elements);
                });
            }
            (OneOf2::B(prev), OneOf2::B(new), OneOf2::B(inner_state)) => {
                ctx.with_id(ViewId::new(seq_state.generation), |ctx| {
                    new.seq_rebuild(prev, inner_state, ctx, elements);
                });
            }
            _ => (),
        };

        // `ViewSequence` has changed type, teardown the old view sequence
        prev.seq_teardown(seq_state, ctx, elements);

        // Overflow handling: u64 starts at 0, incremented by 1 always.
        // Can never realistically overflow, scale is too large.
        // If would overflow, wrap to zero. Would need async message sent
        // to view *exactly* `u64::MAX` versions of the view ago, which is implausible
        seq_state.generation = seq_state.generation.wrapping_add(1);

        // Create the new view sequence

        ctx.with_id(ViewId::new(seq_state.generation), |ctx| {
            match self {
                OneOf2::A(new) => {
                    seq_state.inner_state =
                        OneOf2::A(elements.with_scratch(|elements| new.seq_build(ctx, elements)));
                }
                OneOf2::B(new) => {
                    seq_state.inner_state =
                        OneOf2::B(elements.with_scratch(|elements| new.seq_build(ctx, elements)));
                }
            };
        });
    }

    fn seq_teardown(
        &self,
        seq_state: &mut Self::SeqState,
        ctx: &mut Context,
        elements: &mut impl ElementSplice<Element>,
    ) {
        ctx.with_id(ViewId::new(seq_state.generation), |ctx| {
            match (self, &mut seq_state.inner_state) {
                (OneOf2::A(view), OneOf2::A(state)) => {
                    view.seq_teardown(state, ctx, elements);
                }
                (OneOf2::B(view), OneOf2::B(state)) => {
                    view.seq_teardown(state, ctx, elements);
                }
                _ => unreachable!(),
            }
        });
    }

    fn seq_message(
        &self,
        seq_state: &mut Self::SeqState,
        id_path: &[ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        let (start, rest) = id_path
            .split_first()
            .expect("Id path has elements for OneOf2");
        if start.routing_id() != seq_state.generation {
            // The message was sent to a previous edition of the inner value
            return MessageResult::Stale(message);
        }
        match (self, &mut seq_state.inner_state) {
            (OneOf2::A(view), OneOf2::A(state)) => {
                view.seq_message(state, rest, message, app_state)
            }
            (OneOf2::B(view), OneOf2::B(state)) => {
                view.seq_message(state, rest, message, app_state)
            }
            _ => MessageResult::Stale(message),
        }
    }

    fn count(&self, state: &Self::SeqState) -> usize {
        match (self, &state.inner_state) {
            (OneOf2::A(seq), OneOf2::A(state)) => seq.count(state),
            (OneOf2::B(seq), OneOf2::B(state)) => seq.count(state),
            _ => unreachable!(),
        }
    }
}
