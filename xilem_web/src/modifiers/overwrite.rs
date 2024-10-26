use super::Modifier;

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
    pub(crate) fn new(size_hint: usize) -> Self {
        assert_overflow!(size_hint);
        Self {
            modifiers: 0,
            needs_update: false,
            idx: 0,
            len: 0,
        }
    }

    #[inline]
    pub fn rebuild(&mut self, prev_len: u8) {
        self.idx -= prev_len;
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
    pub fn push(this: &mut Modifier<'_, Self>, modifier: bool) {
        debug_assert!(
            this.flags.was_created(),
            "This should never be called, when the underlying element wasn't (re)created."
        );
        this.modifier.set(modifier);
        this.modifier.needs_update = true;
        this.flags.set_needs_update();
        this.modifier.idx += 1;
        this.modifier.len += 1;
        assert_overflow!(this.modifier.len);
    }

    #[inline]
    /// Mutates the next modifier.
    ///
    /// Must only be used when `self.was_created() == false`.
    pub fn mutate<R>(this: &mut Modifier<'_, Self>, f: impl FnOnce(&mut bool) -> R) -> R {
        debug_assert!(
            !this.flags.was_created(),
            "This should never be called, when the underlying element was (re)created."
        );
        let mut modifier = this.modifier.get();
        let retval = f(&mut modifier);
        let dirty = this.modifier.set(modifier);
        this.modifier.idx += 1;
        this.modifier.needs_update |= this.modifier.len == this.modifier.idx && dirty;
        if this.modifier.needs_update {
            this.flags.set_needs_update();
        }
        retval
    }

    #[inline]
    /// Skips the next `count` modifiers.
    ///
    /// Must only be used when `self.was_created() == false`.
    pub fn skip(this: &mut Modifier<'_, Self>, count: u8) {
        debug_assert!(
            !this.flags.was_created(),
            "This should never be called, when the underlying element was (re)created."
        );
        this.modifier.idx += count;
    }

    #[inline]
    /// Updates the next modifier, based on the diff of `prev` and `next`.
    ///
    /// It can also be used when the underlying element was recreated.
    pub fn update(this: &mut Modifier<'_, Self>, prev: bool, next: bool) {
        if this.flags.was_created() {
            Self::push(this, next);
        } else if next != prev {
            Self::mutate(this, |modifier| *modifier = next);
        } else {
            Self::skip(this, 1);
        }
    }

    #[inline]
    /// Applies potential changes with `f`.
    ///
    /// First argument of `f` is `in_hydration`, the second the new value, if it is set (i.e. is `Some(_)`). When previously modifiers existed, but were deleted it uses `None` as value.
    pub fn apply_changes(&mut self, f: impl FnOnce(Option<bool>)) {
        let needs_update = self.needs_update;
        self.needs_update = false;
        if needs_update {
            let bit = 1 << (self.idx - 1);
            let modifier = (self.len > 0).then_some(self.modifiers & bit == bit);
            f(modifier);
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
            pub(crate) fn new(size_hint: usize) -> Self {
                $modifier($crate::modifiers::OverwriteBool::new(size_hint))
            }

            fn as_overwrite_bool_modifier(
                this: $crate::modifiers::Modifier<'_, Self>,
            ) -> $crate::modifiers::Modifier<'_, $crate::modifiers::OverwriteBool> {
                $crate::modifiers::Modifier::new(&mut this.modifier.0, this.flags)
            }

            pub fn apply_changes(&mut self, f: impl FnOnce(Option<bool>)) {
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
                use $crate::modifiers::With;
                let (mut el, state) =
                    ctx.with_size_hint::<super::$modifier, _>(1, |ctx| self.inner.build(ctx));
                let modifier = &mut super::$modifier::as_overwrite_bool_modifier(el.modifier());
                $crate::modifiers::OverwriteBool::push(modifier, self.value);
                (el, state)
            }

            fn rebuild(
                &self,
                prev: &Self,
                view_state: &mut Self::ViewState,
                ctx: &mut $crate::ViewCtx,
                mut element: $crate::core::Mut<Self::Element>,
            ) {
                use $crate::modifiers::With;
                element.modifier().modifier.0.rebuild(1);
                self.inner
                    .rebuild(&prev.inner, view_state, ctx, element.reborrow_mut());
                let mut modifier = super::$modifier::as_overwrite_bool_modifier(element.modifier());
                $crate::modifiers::OverwriteBool::update(&mut modifier, prev.value, self.value);
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
    use crate::props::ElementFlags;

    use super::*;

    #[test]
    fn overwrite_bool_push() {
        let mut modifier = OverwriteBool::new(2);
        let flags = &mut ElementFlags::new(false);
        let m = &mut Modifier::new(&mut modifier, flags);
        assert!(!m.flags.needs_update());
        OverwriteBool::push(m, true);
        assert!(m.flags.needs_update());
        assert_eq!(m.modifier.len, 1);
        OverwriteBool::push(m, false);
        assert!(m.flags.needs_update());
        assert_eq!(m.modifier.len, 2);
        assert_eq!(m.modifier.idx, 2);
        let mut was_applied = false;
        m.modifier.apply_changes(|value| {
            was_applied = true;
            assert_eq!(value, Some(false));
        });
        assert!(was_applied);
        assert!(!m.modifier.needs_update);
    }

    #[test]
    fn overwrite_bool_mutate() {
        let mut modifier = OverwriteBool::new(4);
        let flags = &mut ElementFlags::new(false);
        let m = &mut Modifier::new(&mut modifier, flags);
        OverwriteBool::push(m, true);
        OverwriteBool::push(m, false);
        OverwriteBool::push(m, true);
        OverwriteBool::push(m, false);
        assert!(m.modifier.needs_update);
        let mut was_applied = false;
        m.modifier.apply_changes(|value| {
            was_applied = true;
            assert_eq!(value, Some(false));
        });
        assert!(was_applied);
        m.flags.clear();
        assert!(!m.flags.needs_update());
        assert_eq!(m.modifier.len, 4);
        assert_eq!(m.modifier.idx, 4);
        m.modifier.rebuild(4);
        assert_eq!(m.modifier.idx, 0);
        assert_eq!(m.modifier.len, 4);
        OverwriteBool::mutate(m, |first| *first = true);
        assert!(!m.modifier.needs_update);
        OverwriteBool::mutate(m, |second| *second = false);
        assert!(!m.modifier.needs_update);
        OverwriteBool::mutate(m, |third| *third = false);
        assert!(!m.modifier.needs_update);
        OverwriteBool::mutate(m, |fourth| *fourth = true);
        assert!(m.modifier.needs_update);
        let mut was_applied = false;
        m.modifier.apply_changes(|value| {
            was_applied = true;
            assert_eq!(value, Some(true));
        });
        assert!(was_applied);
        m.flags.clear();
        assert!(!m.modifier.needs_update);
        assert_eq!(m.modifier.len, 4);
        assert_eq!(m.modifier.idx, 4);
    }

    #[test]
    fn overwrite_bool_skip() {
        let mut modifier = OverwriteBool::new(3);
        let flags = &mut ElementFlags::new(false);
        let m = &mut Modifier::new(&mut modifier, flags);
        OverwriteBool::push(m, true);
        OverwriteBool::push(m, false);
        OverwriteBool::push(m, false);
        let mut was_applied = false;
        m.modifier.apply_changes(|value| {
            was_applied = true;
            assert_eq!(value, Some(false));
        });
        assert!(was_applied);
        m.flags.clear();
        assert!(!m.modifier.needs_update);

        assert_eq!(m.modifier.idx, 3);
        m.modifier.rebuild(3);
        assert_eq!(m.modifier.len, 3);
        assert_eq!(m.modifier.idx, 0);
        OverwriteBool::mutate(m, |first| *first = false); // is overwritten, so don't dirty-flag this.
        assert_eq!(m.modifier.idx, 1);
        assert!(!m.modifier.needs_update);
        OverwriteBool::skip(m, 2);
        assert_eq!(m.modifier.len, 3);
        assert_eq!(m.modifier.idx, 3);
        assert!(!m.modifier.needs_update);
        let mut was_applied = false;
        m.modifier.apply_changes(|value| {
            was_applied = true;
            assert_eq!(value, Some(false));
        });
        m.flags.clear();
        // don't apply if nothing has changed...
        assert!(!was_applied);
    }

    #[test]
    fn overwrite_bool_update() {
        let mut modifier = OverwriteBool::new(3);
        let flags = &mut ElementFlags::new(false);
        let m = &mut Modifier::new(&mut modifier, flags);
        OverwriteBool::push(m, true);
        OverwriteBool::push(m, false);
        OverwriteBool::push(m, false);
        let mut was_applied = false;
        m.modifier.apply_changes(|value| {
            was_applied = true;
            assert_eq!(value, Some(false));
        });
        m.flags.clear();
        assert!(was_applied);
        assert!(!m.modifier.needs_update);

        assert_eq!(m.modifier.idx, 3);
        m.modifier.rebuild(3);
        assert_eq!(m.modifier.len, 3);
        assert_eq!(m.modifier.idx, 0);
        assert_eq!(m.modifier.modifiers, 1);
        // on rebuild
        OverwriteBool::update(m, true, false);
        assert_eq!(m.modifier.idx, 1);
        assert_eq!(m.modifier.modifiers, 0);
        assert!(!m.modifier.needs_update);
        OverwriteBool::update(m, false, true);
        assert_eq!(m.modifier.idx, 2);
        assert_eq!(m.modifier.modifiers, 1 << 1);
        assert!(!m.modifier.needs_update);
        OverwriteBool::update(m, false, true);
        assert_eq!(m.modifier.modifiers, 3 << 1);
        assert_eq!(m.modifier.idx, 3);
        assert!(m.modifier.needs_update);
        let mut was_applied = false;
        m.modifier.apply_changes(|value| {
            was_applied = true;
            assert_eq!(value, Some(true));
        });
        m.flags.clear();
        assert!(was_applied);

        // test recreation
        let mut modifier = OverwriteBool::new(3);
        let flags = &mut ElementFlags::new(false);
        let modifier = &mut Modifier::new(&mut modifier, flags);
        assert_eq!(modifier.modifier.len, 0);
        assert_eq!(modifier.modifier.idx, 0);
        OverwriteBool::update(modifier, false, true);
        assert_eq!(modifier.modifier.idx, 1);
        assert_eq!(modifier.modifier.modifiers, 1);
        assert!(modifier.modifier.needs_update);
        OverwriteBool::update(modifier, true, false);
        OverwriteBool::update(modifier, true, false);
        assert_eq!(modifier.modifier.len, 3);
        assert_eq!(modifier.modifier.idx, 3);
        assert_eq!(modifier.modifier.modifiers, 1);
        let mut was_applied = false;
        modifier.modifier.apply_changes(|value| {
            was_applied = true;
            assert_eq!(value, Some(false));
        });
        modifier.flags.clear();
        assert!(was_applied);
    }

    #[test]
    #[should_panic(
        expected = "This should never be called, when the underlying element was (re)created."
    )]
    fn panic_if_use_mutate_on_creation() {
        let mut modifier = OverwriteBool::new(4);
        let flags = &mut ElementFlags::new(false);
        let m = &mut Modifier::new(&mut modifier, flags);
        assert!(m.flags.was_created());
        OverwriteBool::mutate(m, |m| *m = false);
    }

    #[test]
    #[should_panic(
        expected = "This should never be called, when the underlying element wasn't (re)created."
    )]
    fn panic_if_use_push_on_rebuild() {
        let mut modifier = OverwriteBool::new(4);
        let flags = &mut ElementFlags::new(false);
        let m = &mut Modifier::new(&mut modifier, flags);
        assert!(m.flags.was_created());
        OverwriteBool::push(m, true);
        let mut was_applied = false;
        m.modifier.apply_changes(|value| {
            was_applied = true;
            assert_eq!(value, Some(true));
        });
        assert!(was_applied);
        m.flags.clear();
        assert!(!m.flags.was_created());
        OverwriteBool::push(m, true);
    }
}
