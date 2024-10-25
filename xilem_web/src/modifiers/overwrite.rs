const IN_HYDRATION: u8 = 1 << 0;
const WAS_CREATED: u8 = 1 << 1;

/// A specialized optimized boolean overwrite modifier, with support of up to 32 boolean elements, to avoid allocations.
///
/// It is intended for just overwriting a possibly previous set value.
/// For example `input_el.checked(true).checked(false)` will set the `checked` attribute to `false`.
/// This will usually be used for boolean attributes such as `input.checked`, or `button.disabled`
/// If used for more than 32 it will panic.
pub struct OverwriteBool {
    /// The underlying boolean flags encoded as bitflags.
    modifiers: u32,
    /// the current index of the modifiers. After reconciliation it should equal `len`. In rebuild it is decremented with [`OverwriteBool::rebuild`]
    pub(crate) idx: u8,
    /// The amount of boolean flags used in this modifier.
    len: u8,
    /// To avoid an extra alignment, the boolean flags [`IN_HYDRATION`] and [`WAS_CREATED`] are packed here together.
    flags: u8,
    /// A dirty flag, to indicate whether `apply_changes` should update (i.e. call `f`) the underlying value.
    needs_update: bool,
}

macro_rules! assert_overflow {
    ($index: expr) => {
        debug_assert!(
            $index < 32,
            "If you ever see this,\
             please open an issue at https://github.com/linebender/xilem/, \
             we would appreciate to know what caused this. There are known solutions.\
             This is currently limited to 32 booleans to be more efficient."
        );
    };
}

impl OverwriteBool {
    /// Creates a new `Attributes` modifier.
    ///
    /// `size_hint` is used to avoid unnecessary allocations while traversing up the view-tree when adding modifiers in [`View::build`].
    pub(crate) fn new(size_hint: usize, in_hydration: bool) -> Self {
        assert_overflow!(size_hint);
        let mut flags = WAS_CREATED;
        if in_hydration {
            flags |= IN_HYDRATION;
        }
        Self {
            modifiers: 0,
            needs_update: false,
            flags,
            idx: 0,
            len: 0,
        }
    }

    #[inline]
    pub fn rebuild(&mut self, prev_len: u8) {
        self.idx -= prev_len;
    }

    #[inline]
    /// Returns whether the underlying element has been built or rebuilt, this could e.g. happen, when `OneOf` changes a variant to a different element.
    pub fn was_created(&self) -> bool {
        self.flags & WAS_CREATED != 0
    }

    #[inline]
    /// Returns whether the underlying element has been built or rebuilt, this could e.g. happen, when `OneOf` changes a variant to a different element.
    pub fn in_hydration(&self) -> bool {
        self.flags & IN_HYDRATION != 0
    }

    /// Returns if `modifier` has changed.
    fn set(&mut self, modifier: bool) -> bool {
        let before = self.modifiers & (1 << self.idx);
        if modifier {
            self.modifiers |= 1 << self.idx;
        } else {
            self.modifiers &= !(1 << self.idx);
        }
        (self.modifiers & (1 << self.idx)) != before
    }

    /// Returns the current boolean modifier (at `self.idx`)
    fn get(&self) -> bool {
        let bit = 1 << self.idx;
        self.modifiers & bit == bit
    }

    #[inline]
    /// Pushes `modifier` at the end of the current modifiers.
    ///
    /// Must only be used when `self.was_created() == true`.
    pub fn push(&mut self, modifier: bool) {
        debug_assert!(
            self.was_created(),
            "This should never be called, when the underlying element wasn't (re)created."
        );
        self.set(modifier);
        self.needs_update = true;
        self.idx += 1;
        self.len += 1;
        assert_overflow!(self.len);
    }

    #[inline]
    /// Mutates the next modifier.
    ///
    /// Must only be used when `self.was_created() == false`.
    pub fn mutate<R>(&mut self, f: impl FnOnce(&mut bool) -> R) -> R {
        debug_assert!(
            !self.was_created(),
            "This should never be called, when the underlying element was (re)created."
        );
        let mut modifier = self.get();
        let retval = f(&mut modifier);
        let dirty = self.set(modifier);
        self.idx += 1;
        self.needs_update |= self.len == self.idx && dirty;
        retval
    }

    #[inline]
    /// Skips the next `count` modifiers.
    ///
    /// Must only be used when `self.was_created() == false`.
    pub fn skip(&mut self, count: u8) {
        debug_assert!(
            !self.was_created(),
            "This should never be called, when the underlying element was (re)created."
        );
        self.idx += count;
    }

    #[inline]
    /// Updates the next modifier, based on the diff of `prev` and `next`.
    ///
    /// It can also be used when the underlying element was recreated.
    pub fn update(&mut self, prev: bool, next: bool) {
        if self.was_created() {
            self.push(next);
        } else if next != prev {
            self.mutate(|modifier| *modifier = next);
        } else {
            self.skip(1);
        }
    }

    #[inline]
    /// Applies potential changes with `f`.
    ///
    /// First argument of `f` is `in_hydration`, the second the new value, if it is set (i.e. is `Some(_)`). When previously modifiers existed, but were deleted it uses `None` as value.
    pub fn apply_changes(&mut self, f: impl FnOnce(bool, Option<bool>)) {
        self.flags = 0;
        let needs_update = self.needs_update;
        self.needs_update = false;
        if needs_update {
            let bit = 1 << (self.idx - 1);
            let modifier = (self.len > 0).then_some(self.modifiers & bit == bit);
            f(self.in_hydration(), modifier);
        }
    }
    // TODO implement delete etc.
}

#[macro_export]
/// A macro to create a boolean attribute modifier.
macro_rules! overwrite_bool_modifier {
    ($modifier: ident) => {
        pub struct $modifier($crate::modifiers::OverwriteBool);

        impl $modifier {
            pub(crate) fn new(size_hint: usize, in_hydration: bool) -> Self {
                $modifier($crate::modifiers::OverwriteBool::new(
                    size_hint,
                    in_hydration,
                ))
            }

            pub fn apply_changes(&mut self, f: impl FnOnce(bool, Option<bool>)) {
                self.0.apply_changes(f);
            }
        }
    };
}

#[macro_export]
/// A macro to create a boolean attribute modifier view for a modifier that's in the parent module with the same name,
macro_rules! overwrite_bool_modifier_view {
    ($modifier: ident) => {
        pub struct $modifier<V, State, Action> {
            value: bool,
            inner: V,
            phantom: std::marker::PhantomData<fn() -> (State, Action)>,
        }

        impl<V, State, Action> $modifier<V, State, Action> {
            pub fn new(inner: V, value: bool) -> Self {
                $modifier {
                    inner,
                    value,
                    phantom: std::marker::PhantomData,
                }
            }
        }

        impl<V, State, Action> $crate::core::ViewMarker for $modifier<V, State, Action> {}
        impl<V, State, Action>
            $crate::core::View<State, Action, $crate::ViewCtx, $crate::DynMessage>
            for $modifier<V, State, Action>
        where
            State: 'static,
            Action: 'static,
            V: $crate::DomView<State, Action, Element: $crate::modifiers::With<super::$modifier>>,
            for<'a> <V::Element as $crate::core::ViewElement>::Mut<'a>:
                $crate::modifiers::With<super::$modifier>,
        {
            type Element = V::Element;

            type ViewState = V::ViewState;

            fn build(&self, ctx: &mut $crate::ViewCtx) -> (Self::Element, Self::ViewState) {
                let (mut el, state) =
                    ctx.with_size_hint::<super::$modifier, _>(1, |ctx| self.inner.build(ctx));
                el.modifier().0.push(self.value);
                (el, state)
            }

            fn rebuild(
                &self,
                prev: &Self,
                view_state: &mut Self::ViewState,
                ctx: &mut $crate::ViewCtx,
                mut element: $crate::core::Mut<Self::Element>,
            ) {
                element.modifier().0.rebuild(1);
                self.inner
                    .rebuild(&prev.inner, view_state, ctx, element.reborrow_mut());
                element.modifier().0.update(prev.value, self.value);
            }

            fn teardown(
                &self,
                view_state: &mut Self::ViewState,
                ctx: &mut $crate::ViewCtx,
                element: $crate::core::Mut<Self::Element>,
            ) {
                self.inner.teardown(view_state, ctx, element);
            }

            fn message(
                &self,
                view_state: &mut Self::ViewState,
                id_path: &[$crate::core::ViewId],
                message: $crate::DynMessage,
                app_state: &mut State,
            ) -> $crate::core::MessageResult<Action, $crate::DynMessage> {
                self.inner.message(view_state, id_path, message, app_state)
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overwrite_bool_push() {
        let mut modifier = OverwriteBool::new(2, false);
        assert!(!modifier.needs_update);
        modifier.push(true);
        assert!(modifier.needs_update);
        assert_eq!(modifier.len, 1);
        modifier.push(false);
        assert!(modifier.needs_update);
        assert_eq!(modifier.len, 2);
        assert_eq!(modifier.idx, 2);
        let mut was_applied = false;
        modifier.apply_changes(|_, value| {
            was_applied = true;
            assert_eq!(value, Some(false));
        });
        assert!(was_applied);
        assert!(!modifier.needs_update);
    }

    #[test]
    fn overwrite_bool_mutate() {
        let mut modifier = OverwriteBool::new(4, false);
        modifier.push(true);
        modifier.push(false);
        modifier.push(true);
        modifier.push(false);
        assert!(modifier.needs_update);
        let mut was_applied = false;
        modifier.apply_changes(|_, value| {
            was_applied = true;
            assert_eq!(value, Some(false));
        });
        assert!(was_applied);
        assert!(!modifier.needs_update);
        assert_eq!(modifier.len, 4);
        assert_eq!(modifier.idx, 4);
        modifier.rebuild(4);
        assert_eq!(modifier.idx, 0);
        assert_eq!(modifier.len, 4);
        modifier.mutate(|first| *first = true);
        assert!(!modifier.needs_update);
        modifier.mutate(|second| *second = false);
        assert!(!modifier.needs_update);
        modifier.mutate(|third| *third = false);
        assert!(!modifier.needs_update);
        modifier.mutate(|fourth| *fourth = true);
        assert!(modifier.needs_update);
        let mut was_applied = false;
        modifier.apply_changes(|_, value| {
            was_applied = true;
            assert_eq!(value, Some(true));
        });
        assert!(was_applied);
        assert!(!modifier.needs_update);
        assert_eq!(modifier.len, 4);
        assert_eq!(modifier.idx, 4);
    }

    #[test]
    fn overwrite_bool_skip() {
        let mut modifier = OverwriteBool::new(3, false);
        modifier.push(true);
        modifier.push(false);
        modifier.push(false);
        let mut was_applied = false;
        modifier.apply_changes(|_, value| {
            was_applied = true;
            assert_eq!(value, Some(false));
        });
        assert!(was_applied);
        assert!(!modifier.needs_update);

        assert_eq!(modifier.idx, 3);
        modifier.rebuild(3);
        assert_eq!(modifier.len, 3);
        assert_eq!(modifier.idx, 0);
        modifier.mutate(|first| *first = false); // is overwritten, so don't dirty-flag this.
        assert_eq!(modifier.idx, 1);
        assert!(!modifier.needs_update);
        modifier.skip(2);
        assert_eq!(modifier.len, 3);
        assert_eq!(modifier.idx, 3);
        assert!(!modifier.needs_update);
        let mut was_applied = false;
        modifier.apply_changes(|_, value| {
            was_applied = true;
            assert_eq!(value, Some(false));
        });
        // don't apply if nothing has changed...
        assert!(!was_applied);
    }

    #[test]
    fn overwrite_bool_update() {
        let mut modifier = OverwriteBool::new(3, false);
        modifier.push(true);
        modifier.push(false);
        modifier.push(false);
        let mut was_applied = false;
        modifier.apply_changes(|_, value| {
            was_applied = true;
            assert_eq!(value, Some(false));
        });
        assert!(was_applied);
        assert!(!modifier.needs_update);

        assert_eq!(modifier.idx, 3);
        modifier.rebuild(3);
        assert_eq!(modifier.len, 3);
        assert_eq!(modifier.idx, 0);
        assert_eq!(modifier.modifiers, 1);
        // on rebuild
        modifier.update(true, false);
        assert_eq!(modifier.idx, 1);
        assert_eq!(modifier.modifiers, 0);
        assert!(!modifier.needs_update);
        modifier.update(false, true);
        assert_eq!(modifier.idx, 2);
        assert_eq!(modifier.modifiers, 1 << 1);
        assert!(!modifier.needs_update);
        modifier.update(false, true);
        assert_eq!(modifier.modifiers, 3 << 1);
        assert_eq!(modifier.idx, 3);
        assert!(modifier.needs_update);
        let mut was_applied = false;
        modifier.apply_changes(|_, value| {
            was_applied = true;
            assert_eq!(value, Some(true));
        });
        assert!(was_applied);

        // test recreation
        let mut modifier = OverwriteBool::new(3, false);
        assert_eq!(modifier.len, 0);
        assert_eq!(modifier.idx, 0);
        modifier.update(false, true);
        assert_eq!(modifier.idx, 1);
        assert_eq!(modifier.modifiers, 1);
        assert!(modifier.needs_update);
        modifier.update(true, false);
        modifier.update(true, false);
        assert_eq!(modifier.len, 3);
        assert_eq!(modifier.idx, 3);
        assert_eq!(modifier.modifiers, 1);
        let mut was_applied = false;
        modifier.apply_changes(|_, value| {
            was_applied = true;
            assert_eq!(value, Some(false));
        });
        assert!(was_applied);
    }

    #[test]
    #[should_panic(
        expected = "This should never be called, when the underlying element was (re)created."
    )]
    fn panic_if_use_mutate_on_creation() {
        let mut modifier = OverwriteBool::new(4, false);
        assert!(modifier.was_created());
        modifier.mutate(|m| *m = false);
    }

    #[test]
    #[should_panic(
        expected = "This should never be called, when the underlying element wasn't (re)created."
    )]
    fn panic_if_use_push_on_rebuild() {
        let mut modifier = OverwriteBool::new(4, false);
        assert!(modifier.was_created());
        modifier.push(true);
        let mut was_applied = false;
        modifier.apply_changes(|_, value| {
            was_applied = true;
            assert_eq!(value, Some(true));
        });
        assert!(was_applied);
        assert!(!modifier.was_created());
        modifier.push(true);
    }
}
