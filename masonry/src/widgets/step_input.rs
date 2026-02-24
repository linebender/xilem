// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;
use std::fmt::Debug;

use accesskit::{Action, Node, Role};
use masonry_core::anymore::AnyDebug;
use masonry_core::debug_panic;
use vello::Scene;

use crate::core::keyboard::{Code, Key, NamedKey};
use crate::core::{
    AccessCtx, AccessEvent, ChildrenIds, CursorIcon, EventCtx, LayoutCtx, MeasureCtx, PaintCtx,
    PointerButton, PointerEvent, PropertiesMut, PropertiesRef, Property, PropertySet, QueryCtx,
    RegisterCtx, TextEvent, Update, UpdateCtx, Widget, WidgetMut, WidgetPod,
};
use crate::kurbo::{Affine, Axis, BezPath, Cap, Line, Point, Size, Stroke};
use crate::layout::{LayoutSize, LenReq, Length, SizeDef};
use crate::peniko::{Fill, Gradient};
use crate::properties::{
    BackwardColor, ContentColor, DisabledContentColor, ForwardColor, HeatColor, StepInputStyle,
};
use crate::theme;
use crate::widgets::Label;

/// How much accidental sliding is allowed to still consider it a simple click.
///
/// 3 logical pixels is great for mouse and trackpad,
/// but a larger value should be used for touch.
const SLIDE_INACTIVE_LENGTH: Length = Length::const_px(3.);

// We have three gears with increasing ratios.
const GEAR_1_RATIO: f64 = 1.;
const GEAR_2_RATIO: f64 = 10.;
const GEAR_3_RATIO: f64 = 100.;

// The active gear is chosen based on the distance traveled
const GEAR_1_END: f64 = 100.; // 0 .. 100px
const GEAR_2_END: f64 = 200.; // 100px .. 200px
// Gear 3 is 200px .. infinity

// Calculate total steps at the end of the lower gears
const GEAR_1_END_STEPS: f64 = GEAR_1_RATIO * GEAR_1_END;
const GEAR_2_END_STEPS: f64 = GEAR_2_RATIO * (GEAR_2_END - GEAR_1_END) + GEAR_1_END_STEPS;

/// Defines how many ticks there are per a single step.
///
/// A value of `10` allows for 0.1x speed sliding to still register in ticks.
const TICKS_PER_STEP: i64 = 10;

/// An input widget that steps through values.
///
/// It has increment/decrement buttons for single step movements.
/// For larger changes there is feature-rich pointer control.
/// Click and drag the widget and the value will start changing based on the distance moved.
///
/// There are three different speed zones based on distance:
/// 1. 0 .. 100px moved = 1x speed
/// 2. 100px .. 200px moved = 10x speed
/// 3. 200px .. = 100x speed
///
/// There are two modifier keys to adjust the speed:
/// * Hold `Shift` to increase the speed by 10x
/// * Hold `Alt` to decrease the speed by 10x
///
/// Hold `Ctrl`/`Cmd` to enter into snap mode where the active value
/// will be a multiple of the configured snap value.
///
/// There will be a [`Step`] action when the active value changes due to user input.
///
/// Anything that implements [`Steppable`] can be stepped through.
/// It is already implemented for signed and unsigned integers and floats.
/// For many custom scenarios you can just use an already supported number type
/// and supplement it with a custom display function via [`with_display`].
///
/// [`with_display`]: Self::with_display
pub struct StepInput<T> {
    /// The base value that we're stepping from.
    ///
    /// This is set to the original input value when the widget is initialized,
    /// and is also updated whenever an explicit value update happens.
    /// Crucially, this is never updated by the step logic to avoid error accumulation.
    base: T,
    /// The step size.
    step: T,
    /// The snap interval.
    snap: Option<T>,
    /// The minimum value that can be stepped to.
    min: T,
    /// The maximum value that can be stepped to.
    max: T,

    /// The currently active value.
    value: T,

    /// Ticks determine the active value when combined with the `base` value.
    ///
    /// [`TICKS_PER_STEP`] defines how many ticks are needed for a single `step`.
    ///
    /// While `i64` may not be enough to represent the whole distance from `base` to `min`/`max`,
    /// it is more than enough for UI purposes. Given a person with a ridiculously high
    /// pointer sensitivity that can move 10,000 logical pixels per arm swing. Plus an effective
    /// gear ratio of 10,000,000 ticks per pixel. It would still take that person 3 years
    /// of 24/7 swinging of the arm at a pace of once per second to reach the 63 bit limit.
    ticks: i64,
    /// Number of ticks that achieves the `min` value.
    ///
    /// Saturated at `i64::MIN` if reaching `min` from `base` actually requires more.
    min_ticks: i64,
    /// Number of ticks that achieves the `max` value.
    ///
    /// Saturated at `i64::MAX` if reaching `max` from `base` actually requires more.
    max_ticks: i64,

    /// Whether the values should form an infinite circle,
    /// where increasing above `max` results in `min` and
    /// decreasing below `min` results in `max`.
    wrap: bool,

    /// The label displaying the current active value.
    label: Option<WidgetPod<Label>>,
    /// Custom display function override.
    display: Option<DisplayFn<T>>,

    /// The pointer position where the button was pressed down.
    drag_start: Option<Point>,
    /// The pointer position where ticks were last updated.
    slide_last: Option<Point>,
    /// Whether slower slide mode is enabled.
    slide_slower: bool,
    /// Whether faster slide mode is enabled.
    slide_faster: bool,

    /// When hovered, `true` means the backward side and `false` the forward side.
    hover_backward: bool,

    /// Cached label layout x origin.
    label_x_start: f64,
    /// Cached label layout x endpoint.
    label_x_end: f64,
}

/// The [`StepInput`] context.
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
pub struct StepState<T> {
    /// The currently active value.
    pub value: T,
    /// The step size.
    pub step: T,
    /// The minimum value that can be stepped to.
    pub min: T,
    /// The maximum value that can be stepped to.
    pub max: T,
}

/// Returns the string representation of this value.
///
/// Takes a [`StepState`] and the returned value will be used for display purposes.
pub type DisplayFn<T> = Box<dyn Fn(StepState<T>) -> String>;

/// Data that can be stepped through in a [`StepInput`].
pub trait Steppable: Debug + Copy + Clone + PartialEq + PartialOrd + Send + Sync + 'static {
    /// Returns `self` with the given number of steps added.
    ///
    /// The addition needs to be saturating and not overflow.
    ///
    /// This method is always called on the base value,
    /// so there is no risk of cumulative addings causing errors.
    ///
    /// If `snap` is present then the returned value should be snapped to it.
    /// Which is to say, `result % snap` should be zero. With the exception
    /// of min/max boundaries of the data type being reached.
    fn add_steps(&self, step: Self, count: i64, snap: Option<Self>) -> Self;

    /// Returns the number of steps required to reach `target` from `self`.
    ///
    /// The `target` can be safely assumed to always be equal or greater than `self`.
    ///
    /// The number of steps needs to reach `target` exactly or go beyond it.
    /// For example if `self` is `5`, `target` is `17`, and `step` is `5`,
    /// then the correct result is `3`.
    ///
    /// If the required steps to reach `target` doesn't fit in `u64`,
    /// then `u64::MAX` must be returned. Do not overflow to `u64::MIN`.
    fn steps_to(&self, target: Self, step: Self) -> u64;

    /// Returns the minimum, greater than zero, step value.
    ///
    /// This is used to validate that both the step and snap are greater than zero.
    /// It is used as the fallback value when a value less than this is encountered.
    fn min_step() -> Self;

    /// Returns the string representation of this value.
    ///
    /// The returned value will be used for display purposes.
    ///
    /// The `min`, `max`, `step` values are available as optional hints.
    /// For example, the default `f64` implementation determines
    /// how many decimal places to show based on the step size.
    fn display(&self, step: Self, min: Self, max: Self) -> String;
}

// --- MARK: IMPL STEPPABLE

/// Allows calling `saturating_add_unsigned` even on unsigned types.
///
/// For example `saturating_add_unsigned!(5u8, 1u8, u) == 6`.
macro_rules! saturating_add_unsigned {
    ($value:expr, $delta:expr, i) => {
        ($value).saturating_add_unsigned($delta)
    };
    ($value:expr, $delta:expr, u) => {
        ($value).saturating_add($delta)
    };
}

/// Allows calling `saturating_sub_unsigned` even on unsigned types.
///
/// For example `saturating_sub_unsigned!(6u8, 1u8, u) == 4`.
macro_rules! saturating_sub_unsigned {
    ($value:expr, $delta:expr, i) => {
        ($value).saturating_sub_unsigned($delta)
    };
    ($value:expr, $delta:expr, u) => {
        ($value).saturating_sub($delta)
    };
}

/// Implement `Steppable` for integer types.
///
/// * `$t` == type on which to implement `Steppable` for
/// * `$u` == the unsigned variant of `$t`
/// * `$s` == signedness of `$t`, `u` or `i`
macro_rules! impl_steppable_int {
    ($(($t:ty, $u:ty, $s:tt)),+) => {
        $(
            impl Steppable for $t {
                #[allow(trivial_numeric_casts, reason = "it's not always trivial")]
                fn add_steps(&self, step: Self, count: i64, snap: Option<Self>) -> Self {
                    // Add the steps
                    // * Use unsigned delta so we can go from signed::MIN -> signed::MAX
                    // * Clamp count to the maximum unsigned value to avoid incorrect truncation
                    const MAX: u64 = <$u>::MAX as u64;
                    let mut value = if count < 0 {
                        let count = count.unsigned_abs().min(MAX) as $u;
                        match (step as $u).checked_mul(count) {
                            Some(delta) => saturating_sub_unsigned!(self, delta, $s),
                            None => Self::MIN,
                        }
                    } else {
                        let count = (count as u64).min(MAX) as $u;
                        match (step as $u).checked_mul(count) {
                            Some(delta) => saturating_add_unsigned!(self, delta, $s),
                            None => Self::MAX,
                        }
                    };

                    // Potential snapping
                    if let Some(snap) = snap {
                        let rem = value.rem_euclid(snap);
                        if rem > 0 {
                            // Check if we are closer to the next step.
                            // If rem >= dist_to_next, we are past the midpoint.
                            let dist_to_next = snap - rem;
                            if rem >= dist_to_next {
                                // Round up: add the missing distance.
                                value = value.saturating_add(dist_to_next);
                            } else {
                                // Round down: subtract the remainder.
                                value = value.saturating_sub(rem);
                            }
                        }
                    }

                    // Return the result
                    value
                }

                #[allow(trivial_numeric_casts, reason = "it's not always trivial")]
                fn steps_to(&self, target: Self, step: Self) -> u64 {
                    let step = step as $u;
                    let delta = self.abs_diff(target);
                    let steps = delta / step;
                    let steps = if delta % step > 0 { steps + 1 } else { steps };
                    steps as u64
                }

                fn min_step() -> Self {
                    1
                }

                fn display(&self, _step: Self, _min: Self, _max: Self) -> String {
                    format!("{self}")
                }
            }
        )+
    };
}

impl_steppable_int!(
    (i8, u8, i),
    (u8, u8, u),
    (i16, u16, i),
    (u16, u16, u),
    (i32, u32, i),
    (u32, u32, u),
    (i64, u64, i),
    (u64, u64, u),
    (isize, usize, i),
    (usize, usize, u)
);

/// Implement `Steppable` for float types.
///
/// * `$t` == type on which to implement `Steppable` for
macro_rules! impl_steppable_float {
    ($($t:ty),+) => {
        $(
            impl Steppable for $t {
                #[allow(trivial_numeric_casts, reason = "it's not always trivial")]
                fn add_steps(&self, step: Self, count: i64, snap: Option<Self>) -> Self {
                    // Add the steps
                    // * Do the math in f64 for more precision in case numbers are bit-heavy
                    // * Casting to small float is ok, overflow won't truncate and will be infinite.
                    let mut value = (*self as f64 + step as f64 * count as f64) as $t;
                    // Potential snapping
                    if let Some(snap) = snap {
                        value = (value / snap).round() * snap;
                    }
                    // Return the result
                    value
                }

                #[allow(trivial_numeric_casts, reason = "it's not always trivial")]
                fn steps_to(&self, target: Self, step: Self) -> u64 {
                    let delta = target as f64 - *self as f64;
                    let steps = delta / step as f64;
                    steps.ceil() as u64
                }

                fn min_step() -> Self {
                    Self::EPSILON
                }

                fn display(&self, step: Self, _min: Self, _max: Self) -> String {
                    let precision = if step < 1. {
                        // 0.XXX => 1
                        // 0.0XX => 2
                        // 0.00X => 3
                        (-step.log10().floor()) as usize
                    } else {
                        0
                    };
                    format!("{self:.0$}", precision)
                }
            }
        )+
    };
}

impl_steppable_float!(f32, f64);

// --- MARK: BUILDERS
impl<T: Steppable> StepInput<T> {
    /// Creates a new [`StepInput`].
    ///
    /// Takes the following inputs:
    /// * `base` - value that we're stepping from.
    ///   `base` must be greater or equal to `min`, or it will fall back to `min`.
    ///   `base` must be less than or equal to `max`, or it will fall back to `max`.
    /// * `step` - positive value added to `base` for each step.
    ///   `step` must be greater than zero, or it will fall back to `T::min_step`.
    /// * `min` - minimum value that can be stepped to.
    ///   `min` must be less than or equal to `max`, or it will fall back to `max`.
    /// * `max` - maximum value that can be stepped to.
    ///
    /// # Panics
    ///
    /// Panics if `min` is greater than `max` and debug assertions are enabled.
    ///
    /// Panics if `base` is less than `min` and debug assertions are enabled.
    ///
    /// Panics if `base` is greater than `max` and debug assertions are enabled.
    ///
    /// Panics if `step` is less than `T::min_step` and debug assertions are enabled.
    pub fn new(mut base: T, mut step: T, mut min: T, max: T) -> Self {
        if min > max {
            debug_panic!("provided `min` must be less than or equal to the provided `max`");
            min = max;
        }
        if base < min {
            debug_panic!("provided `base` must be greater or equal to `min`");
            base = min;
        }
        if base > max {
            debug_panic!("provided `base` must be less than or equal to `max`");
            base = max;
        }
        if step < T::min_step() {
            debug_panic!("provided `step` must be greater than zero and at least `T::min_step`");
            step = T::min_step();
        }

        let mut this = Self {
            base,
            step,
            snap: None,
            min,
            max,
            value: base,
            ticks: 0,
            min_ticks: 0,
            max_ticks: 0,
            wrap: false,
            label: None,
            display: None,
            drag_start: None,
            slide_last: None,
            slide_slower: false,
            slide_faster: false,
            hover_backward: false,
            label_x_start: 0.,
            label_x_end: 0.,
        };

        this.calculate_tick_bounds();

        this
    }

    /// Returns the [`StepInput`] with the given `snap` value.
    ///
    /// When the user activates snap mode by holding `Ctrl`/`Cmd`,
    /// then the step value will be snapped to a multiple of this.
    ///
    /// The `snap` value must be greater than zero. Invalid values will be ignored.
    ///
    /// # Panics
    ///
    /// Panics if `snap` is zero or less and debug assertions are enabled.
    pub fn with_snap(mut self, snap: T) -> Self {
        if snap >= T::min_step() {
            self.snap = Some(snap);
        } else {
            debug_panic!("`snap` must be greater than zero and at least `T::min_step`");
        }
        self
    }

    /// Returns the [`StepInput`] with the given `wrap` value.
    ///
    /// Wrap determines whether the values should form an infinite circle,
    /// where increasing above `max` results in `min` and
    /// decreasing below `min` results in `max`.
    pub fn with_wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }

    /// Returns the [`StepInput`] with a custom `display` function.
    ///
    /// This `display` function will receive a [`StepState`]
    /// and it will need to return its `String` representation.
    ///
    /// This is useful for showing units (1 ft, 2 GB) or for doing visual-only rounding, etc.
    ///
    /// [`StepState`]: StepState
    pub fn with_display<F>(mut self, display: F) -> Self
    where
        F: Fn(StepState<T>) -> String + 'static,
    {
        self.display = Some(Box::new(display));
        self
    }
}

// --- MARK: WIDGETMUT
impl<T: Steppable> StepInput<T> {
    /// Set a new `base` value.
    ///
    /// The new `base` value must be greater or equal to `min`
    /// and less than or equal to `max`. Otherwise it will be ignored.
    ///
    /// Never call this in response to a [`Step`] action as it may cause precision issues.
    /// Only call this when the base value has changed due to reasons other than what
    /// [`StepInput`] already knows about.
    ///
    /// # Panics
    ///
    /// Panics if `base` is less than `min` or greater than `max` and debug assertions are enabled.
    pub fn set_base(this: &mut WidgetMut<'_, Self>, base: T) {
        let modified = this.widget.set_base_impl(base);
        if modified {
            let display_value = this.widget.display_value(base);
            if let Some(label) = this.widget.label.as_mut() {
                this.ctx.mutate_later(label, move |mut label| {
                    Label::set_text(&mut label, display_value);
                });
                this.ctx.request_layout();
            }
        }
    }

    /// Set a new `step` value.
    ///
    /// The new `step` value must be greater than zero. Invalid values will be ignored.
    ///
    /// # Panics
    ///
    /// Panics if `step` is zero or less and debug assertions are enabled.
    pub fn set_step(this: &mut WidgetMut<'_, Self>, step: T) {
        if step < T::min_step() {
            debug_panic!("set_step: `step` >= `T::min_step` assert failed");
            return;
        }
        this.widget.step = step;
        // The step changing means we also need to rebase and reset ticks
        Self::set_base(this, this.widget.value);
    }

    /// Set a new `snap` value.
    ///
    /// The new `snap` value must be greater than zero. Invalid values will be ignored.
    /// `None` disables the snap functionality.
    ///
    /// # Panics
    ///
    /// Panics if `snap` is zero or less and debug assertions are enabled.
    pub fn set_snap(this: &mut WidgetMut<'_, Self>, snap: Option<T>) {
        if let Some(snap) = snap
            && snap < T::min_step()
        {
            debug_panic!("set_snap: `snap` > `T::min_step` assert failed");
            return;
        }
        this.widget.snap = snap;
    }

    /// Set new `min` and `max` bounds.
    ///
    /// `min` must be less or equal to `max`. Invalid values will be ignored.
    ///
    /// # Panics
    ///
    /// Panics if `min` is greater than `max` and debug assertions are enabled.
    pub fn set_bounds(this: &mut WidgetMut<'_, Self>, min: T, max: T) {
        if min > max {
            debug_panic!("set_bounds: `min` <= `max` assert failed");
            return;
        }
        this.widget.min = min;
        this.widget.max = max;
        // Make sure the base value and the active value are within bounds.
        // If not, just rebase at the boundary.
        if this.widget.base < min || this.widget.value < min {
            Self::set_base(this, min);
        } else if this.widget.base > max || this.widget.value > max {
            Self::set_base(this, max);
        } else {
            // All good, but still need to calculate new tick bounds
            this.widget.calculate_tick_bounds();
        }
    }

    /// Set a new `wrap` value.
    ///
    /// Wrap determines whether the values should form an infinite circle,
    /// where increasing above `max` results in `min` and
    /// decreasing below `min` results in `max`.
    pub fn set_wrap(this: &mut WidgetMut<'_, Self>, wrap: bool) {
        this.widget.wrap = wrap;
    }

    /// Set a new custom `display` function.
    ///
    /// This `display` function will receive a [`StepState`]
    /// and it will need to return its `String` representation.
    ///
    /// This is useful for showing units (1 ft, 2 GB) or for doing visual-only rounding, etc.
    ///
    /// [`StepState`]: StepState
    pub fn set_display<F>(this: &mut WidgetMut<'_, Self>, display: F)
    where
        F: Fn(StepState<T>) -> String + 'static,
    {
        this.widget.display = Some(Box::new(display));
        // Refresh the displayed value too
        let display_value = this.widget.display_value(this.widget.value);
        if let Some(label) = this.widget.label.as_mut() {
            this.ctx.mutate_later(label, move |mut label| {
                Label::set_text(&mut label, display_value);
            });
            this.ctx.request_layout();
        }
    }
}

// --- MARK: METHODS
impl<T: Steppable> StepInput<T> {
    /// Returns the string representation of the given `value`.
    fn display_value(&self, value: T) -> String {
        if let Some(display) = self.display.as_deref() {
            let state = StepState {
                value,
                step: self.step,
                min: self.min,
                max: self.max,
            };
            display(state)
        } else {
            value.display(self.step, self.min, self.max)
        }
    }

    /// Updates the label with the new value and emits a [`Step`] action.
    fn handle_updated_value<A: AnyDebug + Send + From<Step<T>>>(&mut self, ctx: &mut EventCtx<'_>) {
        let display_value = self.display_value(self.value);
        if let Some(label) = self.label.as_mut() {
            ctx.mutate_later(label, move |mut label| {
                Label::set_text(&mut label, display_value);
            });
            ctx.request_layout();
        }
        ctx.submit_action::<A>(Step { value: self.value });
    }

    /// Calculates the active value.
    ///
    /// This can return a value outside the allowed bounds.
    /// Use [`clamp_value`] to deal with that.
    ///
    /// [`clamp_value`]: Self::clamp_value
    fn calculate_value(base: T, step: T, snap: Option<T>, ticks: i64) -> T {
        // We round the ticks for smoother sliding and symmetry when going in both directions.
        let step_count = ticks.saturating_add(TICKS_PER_STEP / 2 * ticks.signum()) / TICKS_PER_STEP;
        base.add_steps(step, step_count, snap)
    }

    /// Clamps the given `value` between `min` and `max`.
    fn clamp_value(value: T, min: T, max: T) -> T {
        if value < min {
            min
        } else if value > max {
            max
        } else {
            value
        }
    }

    /// Returns the `steps` as positive ticks.
    fn steps_to_positive_ticks(steps: u64) -> i64 {
        steps
            .saturating_mul(TICKS_PER_STEP as u64)
            .min(i64::MAX as u64)
            .cast_signed()
    }

    /// Returns the `steps` as negative ticks.
    fn steps_to_negative_ticks(steps: u64) -> i64 {
        steps
            .checked_mul(TICKS_PER_STEP as u64)
            .map(|t| {
                if t >= i64::MIN as u64 {
                    i64::MIN
                } else {
                    t.wrapping_neg().cast_signed()
                }
            })
            .unwrap_or(i64::MIN)
    }

    /// Calculates `min_ticks`/`max_ticks` for reaching `min`/`max` from `base` with `step`.
    ///
    /// Note that these results saturate at their limits, so e.g. `i64::MAX` for `max_ticks`
    /// does not necessarily mean that `max` will be reached.
    fn calculate_tick_bounds(&mut self) {
        let min_steps = self.min.steps_to(self.base, self.step);
        self.min_ticks = Self::steps_to_negative_ticks(min_steps);

        let max_steps = self.base.steps_to(self.max, self.step);
        self.max_ticks = Self::steps_to_positive_ticks(max_steps);
    }

    /// Sets a new `base` value and returns `true` if it actually changed anything.
    ///
    /// # Panics
    ///
    /// Panics if base is less than `min` or greater than `max` and debug assertions are enabled.
    fn set_base_impl(&mut self, base: T) -> bool {
        if base < self.min || base > self.max {
            debug_panic!("set_base: `min` <= `base` <= `max` assert failed");
            return false;
        }
        // We do this even if self.base == base, because some callers expect ticks to be reset.
        self.base = base;
        self.value = base;
        self.ticks = 0;
        self.calculate_tick_bounds();
        true
    }

    /// Sets the active value based on the given `ticks`.
    ///
    /// If `snap` is `true` then the value will also be snapped.
    ///
    /// The value will be clamped into the allowed bounds.
    fn set_value(&mut self, ticks: i64, snap: bool) {
        let snap = self.snap.filter(|_| snap);
        let raw_value = Self::calculate_value(self.base, self.step, snap, ticks);
        self.value = Self::clamp_value(raw_value, self.min, self.max);
    }

    /// Sets the active value to the next snapped value.
    ///
    /// May increment ticks to achieve this.
    ///
    /// Returns `true` if the active value changed.
    fn next_snap(&mut self) -> bool {
        let Some(snap) = self.snap else {
            return self.next_step();
        };
        let old_value = self.value;
        let (_, changed, hit_bounds) = self.update_ticks(TICKS_PER_STEP, true);

        if changed && (self.value > old_value || hit_bounds) {
            self.sync_ticks_to_value();
        } else {
            // We need to add ~snap worth of ticks to get the next
            let steps = T::min_step().steps_to(snap, self.step);
            let delta = Self::steps_to_positive_ticks(steps);
            self.update_ticks(delta, true);
        }

        old_value != self.value
    }

    /// Sets the active value to the previous snapped value.
    ///
    /// May decrement ticks to achieve this.
    ///
    /// Returns `true` if the active value changed.
    fn prev_snap(&mut self) -> bool {
        let Some(snap) = self.snap else {
            return self.prev_step();
        };
        let old_value = self.value;
        let (_, changed, hit_bounds) = self.update_ticks(TICKS_PER_STEP.wrapping_neg(), true);

        if changed && (self.value < old_value || hit_bounds) {
            self.sync_ticks_to_value();
        } else {
            // We need to subtract ~snap worth of ticks to get the previous
            let steps = T::min_step().steps_to(snap, self.step);
            let delta = Self::steps_to_negative_ticks(steps);
            self.update_ticks(delta, true);
        }

        old_value != self.value
    }

    /// Sets ticks to the amount needed to go from `base` to `value`.
    fn sync_ticks_to_value(&mut self) {
        if self.value >= self.base {
            let steps = self.base.steps_to(self.value, self.step);
            self.ticks = Self::steps_to_positive_ticks(steps).min(self.max_ticks);
        } else {
            let steps = self.value.steps_to(self.base, self.step);
            self.ticks = Self::steps_to_negative_ticks(steps).max(self.min_ticks);
        }
    }

    /// Updates ticks based on the given `delta`.
    ///
    /// Returns `(unused_delta, value_changed, hit_bounds)`.
    ///
    /// Ticks will be updated to min/max boundaries if wrapping is disabled.
    /// Otherwise the value will be rebased at the wrapping boundary
    /// and ticks will be relative to that new base.
    fn update_ticks(&mut self, delta: i64, snap: bool) -> (i64, bool, bool) {
        if delta == 0 {
            return (0, false, false);
        }

        // Wrapping costs a full step
        const WRAP_COST: u64 = TICKS_PER_STEP as u64;

        let dpos = delta.is_positive();

        let mut new_ticks = self.ticks;
        let mut unused_delta = delta.unsigned_abs();

        loop {
            // Calculate free space before tick bounds
            let space = if dpos {
                (self.max_ticks as u64).wrapping_sub(new_ticks as u64)
            } else {
                (new_ticks as u64).wrapping_sub(self.min_ticks as u64)
            };
            // If unused delta fits in the space, just use it all
            if unused_delta <= space {
                new_ticks = if dpos {
                    (new_ticks as u64).wrapping_add(unused_delta)
                } else {
                    (new_ticks as u64).wrapping_sub(unused_delta)
                }
                .cast_signed();
                unused_delta = 0;
                break;
            }
            // Otherwise we hit tick bounds
            new_ticks = if dpos { self.max_ticks } else { self.min_ticks };
            unused_delta -= space;
            // Perhaps we can wrap?
            if self.wrap && unused_delta >= WRAP_COST {
                self.base = if dpos { self.min } else { self.max };
                self.calculate_tick_bounds();
                new_ticks = 0;
                unused_delta -= WRAP_COST;
            } else {
                break;
            }
        }

        // Convert the unused delta back to signed
        let mut unused_delta = if dpos {
            unused_delta
        } else {
            unused_delta.wrapping_neg()
        }
        .cast_signed();

        let hit_bounds = unused_delta != 0;

        // Update the ticks
        self.ticks = new_ticks;

        // Calculate the active value change
        let old_value = self.value;
        self.set_value(new_ticks, snap);
        let changed_value = old_value != self.value;

        // Keep the ticks at the actual snapped value
        if snap {
            self.sync_ticks_to_value();
            unused_delta += new_ticks - self.ticks;
        }

        (unused_delta, changed_value, hit_bounds)
    }

    /// Increments ticks by one step's worth.
    ///
    /// Returns `true` if the active value changed.
    #[inline(always)]
    fn next_step(&mut self) -> bool {
        self.update_ticks(TICKS_PER_STEP, false).1
    }

    /// Decrements ticks by one step's worth.
    ///
    /// Returns `true` if the active value changed.
    #[inline(always)]
    fn prev_step(&mut self) -> bool {
        self.update_ticks(TICKS_PER_STEP.wrapping_neg(), false).1
    }

    /// Returns slide `(speed, forward, backward)` for visual purposes.
    ///
    /// The speed is linear and normalized `[0..1]`.
    ///
    /// * 80% based on distance compared to max gear requirement
    /// * +10% if slower mode is not enabled
    /// * +10% if faster mode is enabled
    ///
    /// Actual step gearing is unlikely to be linear,
    /// but linear works much better for visual cues.
    ///
    /// If the slide is increasing the value then `forward` is `true`.
    /// If the slide is decreasing the value then `backward` is `true`.
    fn visual_speed(&self) -> (f64, bool, bool) {
        let (Some(drag_start), Some(slide_last)) = (self.drag_start, self.slide_last) else {
            return (0., false, false);
        };

        let distance = slide_last.x - drag_start.x;
        let direction = distance.signum();
        let distance = distance.abs();
        if distance == 0. {
            return (0., false, false);
        }

        let mut speed = (distance / GEAR_2_END).clamp(0., 1.) * 0.8;
        if !self.slide_slower {
            speed += 0.1;
        }
        if self.slide_faster {
            speed += 0.1;
        }
        (speed, direction > 0., direction < 0.)
    }

    /// Returns the number of steps needed to cover the given `distance`.
    ///
    /// The given `distance` must be from the drag start position.
    ///
    /// This function will also return the drift from the given `distance`,
    /// in case the `distance` does not precisely convert into integer steps.
    /// Positive drift means that not all the `distance` was used,
    /// negative drift means that more than `distance` was used.
    ///
    /// We use distance based gearing. The bigger the distance, the faster the steps will grow.
    fn steps(distance: f64) -> (i64, f64) {
        let direction = distance.signum();
        let distance = distance.abs();

        // Calculate the number of steps that the given distance converts to.
        // We do this with a progressive system, to ensure the earlier distance
        // is always covered using the lower gears. Not doing this would introduce pointer
        // movement speed into the formula, as jumps in distance would only use the high gear.
        let (start_distance, base_steps, ratio) = if distance <= GEAR_1_END {
            (0., 0., GEAR_1_RATIO)
        } else if distance <= GEAR_2_END {
            (GEAR_1_END, GEAR_1_END_STEPS, GEAR_2_RATIO)
        } else {
            (GEAR_2_END, GEAR_2_END_STEPS, GEAR_3_RATIO)
        };

        let steps_raw = base_steps + (distance - start_distance) * ratio;
        let steps = steps_raw.round();
        let drift = (steps_raw - steps) * ratio.recip();

        // Return the steps with the correct direction
        ((steps * direction) as i64, drift * direction)
    }

    /// Returns the distance covered by the given `steps`.
    ///
    /// The returned distance is from the drag start position,
    /// so the provided `steps` need to be the total steps from that point.
    ///
    /// This function accounts for distance based gearing.
    /// The bigger the distance, the more steps will be used.
    fn distance(steps: f64) -> f64 {
        let direction = steps.signum();
        let steps = steps.abs();

        let gear_3_steps = (steps - GEAR_2_END_STEPS).max(0.);
        let gear_2_steps = (steps - GEAR_1_END_STEPS - gear_3_steps).max(0.);
        let gear_1_steps = steps - gear_2_steps - gear_3_steps;

        let gear_1_distance = gear_1_steps * GEAR_1_RATIO.recip();
        let gear_2_distance = gear_2_steps * GEAR_2_RATIO.recip();
        let gear_3_distance = gear_3_steps * GEAR_3_RATIO.recip();

        (gear_1_distance + gear_2_distance + gear_3_distance) * direction
    }
}

/// The latest active value of [`StepInput`].
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Step<T> {
    /// Currently active value.
    pub value: T,
}

// --- MARK: IMPL WIDGET
impl<T: Steppable> Widget for StepInput<T> {
    type Action = Step<T>;

    fn on_access_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &AccessEvent,
    ) {
        // TODO: Should we support ctrl/cmd snap modifier here?

        let mut value_changed = false;

        match event.action {
            Action::Increment => {
                value_changed = self.next_step();
            }
            Action::Decrement => {
                value_changed = self.prev_step();
            }
            _ => (),
        }

        // If the value was changed, we need to handle it
        if value_changed {
            self.handle_updated_value::<Self::Action>(ctx);
        }
    }

    fn on_text_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &TextEvent,
    ) {
        match event {
            TextEvent::Keyboard(ke) => {
                let mut value_changed = false;

                // Snap based on modifier
                let snap = if cfg!(target_os = "macos") {
                    ke.modifiers.meta()
                } else {
                    ke.modifiers.ctrl()
                };

                match ke.key {
                    // Update slide modifier so the visuals can change without pointer movement.
                    Key::Named(NamedKey::Shift) => {
                        let slide_faster = ke.state.is_down();
                        if slide_faster != self.slide_faster {
                            self.slide_faster = slide_faster;
                            ctx.request_paint_only();
                        }
                    }
                    // Update slide modifier so the visuals can change without pointer movement.
                    Key::Named(NamedKey::Alt) => {
                        let slide_slower = ke.state.is_down();
                        if slide_slower != self.slide_slower {
                            self.slide_slower = slide_slower;
                            ctx.request_paint_only();
                        }
                    }
                    Key::Named(NamedKey::ArrowLeft) | Key::Named(NamedKey::ArrowDown) => {
                        if ke.state.is_down() {
                            value_changed = if snap {
                                self.prev_snap()
                            } else {
                                self.prev_step()
                            }
                        }
                    }
                    Key::Named(NamedKey::ArrowRight) | Key::Named(NamedKey::ArrowUp) => {
                        if ke.state.is_down() {
                            value_changed = if snap {
                                self.next_snap()
                            } else {
                                self.next_step()
                            }
                        }
                    }
                    Key::Named(NamedKey::Home) => {
                        if ke.state.is_down() {
                            value_changed = self.set_base_impl(self.min);
                        }
                    }
                    Key::Named(NamedKey::End) => {
                        if ke.state.is_down() {
                            value_changed = self.set_base_impl(self.max);
                        }
                    }
                    _ => match ke.code {
                        Code::NumpadSubtract => {
                            if ke.state.is_down() {
                                value_changed = if snap {
                                    self.prev_snap()
                                } else {
                                    self.prev_step()
                                }
                            }
                        }
                        Code::NumpadAdd => {
                            if ke.state.is_down() {
                                value_changed = if snap {
                                    self.next_snap()
                                } else {
                                    self.next_step()
                                }
                            }
                        }
                        _ => (),
                    },
                }

                // If the value was changed, we need to handle it
                if value_changed {
                    self.handle_updated_value::<Self::Action>(ctx);
                }
            }
            _ => (),
        }
    }

    fn on_pointer_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &PointerEvent,
    ) {
        match event {
            PointerEvent::Down(pbe)
                if pbe.button.is_none_or(|btn| btn == PointerButton::Primary) =>
            {
                // TODO: Once we get editable input support,
                //       stop starting the slide from the edit area.

                let pos = pbe.state.logical_point();
                self.drag_start = Some(pos);
                ctx.capture_pointer();
                ctx.request_focus();

                // TODO: Lock and hide the pointer during a slide.
                //       https://github.com/linebender/xilem/issues/850
            }
            PointerEvent::Move(pu) => {
                // If we're hovered, highlight the correct side's button.
                if ctx.is_hovered() {
                    let size = ctx.content_box_size();
                    let local_x = ctx.local_position(pu.current.position).x;
                    let hover_backward = local_x <= size.width * 0.5;
                    if hover_backward != self.hover_backward {
                        self.hover_backward = hover_backward;
                        ctx.request_paint_only();
                    }
                }

                // Unless a drag is in progress we can return early.
                let Some(drag_start) = self.drag_start else {
                    return;
                };

                // We do slide math in logical pixels, because we don't want
                // the scale factor (i.e. high DPI screens) to affect the speed.
                let pos = pu.current.logical_point();
                let distance = (pos - drag_start).x;

                // Determine the movement delta since the last slide update.
                let distance_last = if let Some(slide_last) = self.slide_last {
                    (slide_last - drag_start).x
                } else {
                    // We don't start the slide with just any pointer drag,
                    // as shaky hands or trackpad use will cause minor inadvertent drag.
                    if distance.abs() - SLIDE_INACTIVE_LENGTH.get() <= 0. {
                        // Not enough drag to start a slide
                        return;
                    }
                    // We're starting the slide now with a slight jump.
                    0.
                };

                // Calculate the steps delta that this latest movement results in.
                let (steps, drift) = Self::steps(distance);
                let (steps_last, _) = Self::steps(distance_last);
                let steps_delta = steps - steps_last;

                let mut ticks_per_step = TICKS_PER_STEP;

                // Modifier based gearing adjusts the ticks per step.
                self.slide_faster = pu.current.modifiers.shift();
                self.slide_slower = pu.current.modifiers.alt();
                if self.slide_faster {
                    ticks_per_step *= 10;
                }
                if self.slide_slower {
                    ticks_per_step /= 10;
                }

                // Calculate the final ticks delta.
                let ticks_delta = steps_delta * ticks_per_step;

                // Pointer move delta can be very granular and we don't want to lose any of it.
                // We also do rounding in steps calculation and need to account for that.
                // So we save the last update position precisely based on what we actually used.
                self.slide_last = Some(Point::new(pos.x - drift, pos.y));

                // Snap based on modifier
                let snap = if cfg!(target_os = "macos") {
                    pu.current.modifiers.meta()
                } else {
                    pu.current.modifiers.ctrl()
                };

                // Update the ticks based on the calculated ticks delta.
                let (ticks_unused, value_changed, hit_bounds) =
                    self.update_ticks(ticks_delta, snap);

                // We calculate unused steps with f64 because unused ticks may not divide nicely.
                let steps_unused = ticks_unused as f64 / ticks_per_step as f64;
                // Used distance here means accounted for, i.e. split between steps and drift.
                let distance_used = Self::distance(steps as f64 - steps_unused) + drift;
                let distance_unused = distance - distance_used;

                if hit_bounds {
                    // Once we reach a min/max edge and there is no wrap,
                    // we need to start shifting the drag_start position towards the pointer.
                    // So that when changing direction we won't be stuck in high gear.
                    self.drag_start =
                        Some(Point::new(drag_start.x + distance_unused, drag_start.y));
                } else {
                    // Otherwise account for the unused distance in the slide.
                    if let Some(slide_last) = self.slide_last.as_mut() {
                        slide_last.x -= distance_unused;
                    }
                }

                // If the value was changed, we need to handle it
                if value_changed {
                    self.handle_updated_value::<Self::Action>(ctx);
                }

                // Always request paint during slide to update the visual speed indicator.
                ctx.request_paint_only();
            }
            PointerEvent::Cancel(_pi) => {
                self.slide_last = None;
                self.drag_start = None;
                ctx.request_paint_only();
            }
            // We only care about primary button and touch
            PointerEvent::Up(pbe) if pbe.button.is_none_or(|btn| btn == PointerButton::Primary) => {
                // Regular click handling happens only if:
                // * There is no slide in progress
                // * The button was previously pressed down on us (active)
                // * The pointer is still on us (hovered)
                if self.slide_last.is_none() && ctx.is_active() && ctx.is_hovered() {
                    let size = ctx.content_box_size();
                    let local_x = ctx.local_position(pbe.state.position).x;

                    // Snap based on modifier
                    let snap = if cfg!(target_os = "macos") {
                        pbe.state.modifiers.meta()
                    } else {
                        pbe.state.modifiers.ctrl()
                    };

                    // Update the active value based on which side was clicked
                    let value_changed = if local_x <= size.width * 0.5 {
                        if snap {
                            self.prev_snap()
                        } else {
                            self.prev_step()
                        }
                    } else if snap {
                        self.next_snap()
                    } else {
                        self.next_step()
                    };

                    // If the value was changed, we need to handle it
                    if value_changed {
                        self.handle_updated_value::<Self::Action>(ctx);
                    }
                }
                self.drag_start = None;
                self.slide_last = None;
                ctx.request_paint_only();
            }
            _ => (),
        }
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        if let Some(label) = self.label.as_mut() {
            ctx.register_child(label);
        }
    }

    fn property_changed(&mut self, ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        if StepInputStyle::matches(property_type) {
            ctx.request_layout();
        } else if ContentColor::matches(property_type)
            || DisabledContentColor::matches(property_type)
            || BackwardColor::matches(property_type)
            || ForwardColor::matches(property_type)
            || HeatColor::matches(property_type)
        {
            ctx.request_paint_only();
        }

        // TODO: Find more elegant way to propagate property to child.
        if ContentColor::matches(property_type) {
            ctx.mutate_self_later(|mut this| {
                let mut this = this.downcast::<Self>();
                let prop = *this.get_prop::<ContentColor>();
                let Some(label) = this.widget.label.as_mut() else {
                    return;
                };
                let mut label = this.ctx.get_mut(label);
                label.insert_prop(prop);
            });
        } else if DisabledContentColor::matches(property_type) {
            ctx.mutate_self_later(|mut this| {
                let mut this = this.downcast::<Self>();
                let prop = this.get_prop_defined::<DisabledContentColor>().copied();
                let Some(label) = this.widget.label.as_mut() else {
                    return;
                };
                let mut label = this.ctx.get_mut(label);
                if let Some(prop) = prop {
                    label.insert_prop(prop);
                } else {
                    label.remove_prop::<DisabledContentColor>();
                }
            });
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, props: &mut PropertiesMut<'_>, event: &Update) {
        match event {
            Update::WidgetAdded => {
                let color = props.get::<ContentColor>();
                let color_disabled = props.get_defined::<DisabledContentColor>();

                let mut props = PropertySet::one(*color);
                if let Some(color_disabled) = color_disabled {
                    props = props.with(*color_disabled);
                }

                let display_value = self.display_value(self.value);
                self.label = Some(Label::new(display_value).with_props(props).to_pod());
                ctx.children_changed();
            }
            Update::ActiveChanged(active) => {
                if !active {
                    self.slide_last = None;
                    self.drag_start = None;
                }
                ctx.request_paint_only();
            }
            Update::DisabledChanged(_) | Update::HoveredChanged(_) => {
                ctx.request_paint_only();
            }
            _ => (),
        }
    }

    fn measure(
        &mut self,
        ctx: &mut MeasureCtx<'_>,
        props: &PropertiesRef<'_>,
        axis: Axis,
        len_req: LenReq,
        cross_length: Option<f64>,
    ) -> f64 {
        // TODO: Remove HACK: Until scale factor rework happens, just pretend it's always 1.0.
        //       https://github.com/linebender/xilem/issues/1264
        let scale = 1.0;

        let content_height = theme::BASIC_WIDGET_HEIGHT.dp(scale);

        let style = props.get::<StepInputStyle>();

        let (len_req, min_result) = match len_req {
            LenReq::MinContent | LenReq::MaxContent => (len_req, 0.),
            // We always want to use up all offered space but may need even more,
            // so we implement FitContent as space.max(MinContent).
            LenReq::FitContent(space) => (LenReq::MinContent, space),
        };

        let calc_button_length = |axis| match axis {
            Axis::Horizontal => match style {
                StepInputStyle::Basic => {
                    let vertical_space = cross_length.unwrap_or(content_height);
                    let (btn_length, btn_edge_pad) =
                        Self::basic_button_length(vertical_space, None);
                    match len_req {
                        LenReq::MinContent => 2. * (btn_length + 2. * btn_edge_pad),
                        LenReq::MaxContent => 2. * (btn_length * 3.),
                        LenReq::FitContent(_) => unreachable!(),
                    }
                }
                StepInputStyle::Flow => {
                    let vertical_space = cross_length.unwrap_or(content_height);
                    let (arrow_width, _, arrow_edge_pad) =
                        Self::flow_button_length(vertical_space, None);
                    match len_req {
                        LenReq::MinContent => 2. * (2. * arrow_width + arrow_edge_pad),
                        LenReq::MaxContent => 2. * (4. * arrow_width + arrow_edge_pad),
                        LenReq::FitContent(_) => unreachable!(),
                    }
                }
            },
            Axis::Vertical => content_height,
        };

        let button_length = calc_button_length(axis);

        let auto_length = match axis {
            Axis::Horizontal => len_req.reduce(button_length).into(),
            Axis::Vertical => len_req.into(),
        };
        let context_size = LayoutSize::maybe(axis.cross(), cross_length);
        let cross = axis.cross();
        let label_cross_length = match cross {
            Axis::Horizontal => {
                cross_length.map(|cross_length| (cross_length - calc_button_length(cross)).max(0.))
            }
            Axis::Vertical => cross_length,
        };
        let label_length = if let Some(label) = self.label.as_mut() {
            ctx.compute_length(label, auto_length, context_size, axis, label_cross_length)
        } else {
            0.
        };

        let length = match axis {
            Axis::Horizontal => label_length + button_length,
            Axis::Vertical => label_length.max(button_length),
        };

        min_result.max(length)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, props: &PropertiesRef<'_>, size: Size) {
        let Some(label) = self.label.as_mut() else {
            return;
        };

        let style = props.get::<StepInputStyle>();

        // Reserve MinContent worth of button space
        let buttons_width = match style {
            StepInputStyle::Basic => {
                let (btn_length, btn_edge_pad) =
                    Self::basic_button_length(size.height, Some(size.width));
                2. * (btn_length + 2. * btn_edge_pad)
            }
            StepInputStyle::Flow => {
                let (arrow_width, _, arrow_edge_pad) =
                    Self::flow_button_length(size.height, Some(size.width));
                2. * (2. * arrow_width + arrow_edge_pad)
            }
        };

        let label_space = Size::new((size.width - buttons_width).max(0.), size.height);
        let auto_size = SizeDef::fit(label_space);
        let label_size = ctx.compute_size(label, auto_size, size.into());
        ctx.run_layout(label, label_size);

        let label_origin = Point::new(
            (size.width - label_size.width) * 0.5,
            (size.height - label_size.height) * 0.5,
        );
        ctx.place_child(label, label_origin);

        self.label_x_start = label_origin.x;
        self.label_x_end = label_origin.x + label_size.width;
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, props: &PropertiesRef<'_>, scene: &mut Scene) {
        match props.get::<StepInputStyle>() {
            StepInputStyle::Basic => Self::paint_basic(self, ctx, props, scene),
            StepInputStyle::Flow => Self::paint_flow(self, ctx, props, scene),
        }
    }

    fn accessibility_role(&self) -> Role {
        Role::SpinButton
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        node: &mut Node,
    ) {
        // NOTE: Can't set numeric value/min/max/step because AccessKit only seems to support f64.

        node.set_value(self.display_value(self.value));
        node.add_action(Action::Increment);
        node.add_action(Action::Decrement);

        // TODO: Add support for SetValue when we gain edit support.
        //       Not trivial, because the value needs to satisfy all rules.
        //node.add_action(Action::SetValue);
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_iter(self.label.as_ref().map(|label| label.id()))
    }

    fn accepts_focus(&self) -> bool {
        true
    }

    fn accepts_text_input(&self) -> bool {
        true
    }

    fn get_cursor(&self, _ctx: &QueryCtx<'_>, _pos: Point) -> CursorIcon {
        CursorIcon::EwResize
    }
}

// --- MARK: PAINT STYLES
impl<T: Steppable> StepInput<T> {
    /// Returns `(length, edge_pad)` of a basic button given the `vertical_space`.
    ///
    /// If total horizontal space is know it will help prevent awkwardly large results.
    ///
    /// The `pad` factor determines how much of vertical space is used for padding on one side.
    fn button_length(vertical_space: f64, horizontal_space: Option<f64>, pad: f64) -> (f64, f64) {
        // We want to always leave a bit of space between the button and the content-box edge,
        // so it won't touch the border even when there is no extra padding in the widget.
        // We base both the padding and the button size itself on the available content-box height.
        let mut edge_pad = pad * vertical_space;
        let mut length = vertical_space - 2. * edge_pad;
        // If we know the total horizontal space, make sure the button length is not over 20% of it.
        // This helps prevent overflow and other awkward results given narrow but tall constraints.
        if let Some(horizontal_space) = horizontal_space {
            let max_length = 0.2 * horizontal_space;
            if length > max_length {
                length = max_length;
                edge_pad = length / (1. - 2. * pad) * pad;
            }
        }
        (length, edge_pad)
    }

    /// Returns `(length, edge_pad)` of a basic button given the `vertical_space`.
    ///
    /// If total horizontal space is know it will help prevent awkwardly large results.
    #[inline(always)]
    fn basic_button_length(vertical_space: f64, horizontal_space: Option<f64>) -> (f64, f64) {
        Self::button_length(vertical_space, horizontal_space, 0.15)
    }

    /// Returns `(width, height, edge_pad)` of a flow arrow given the `vertical_space`.
    ///
    /// If total horizontal space is know it will help prevent awkwardly large results.
    #[inline(always)]
    fn flow_button_length(vertical_space: f64, horizontal_space: Option<f64>) -> (f64, f64, f64) {
        let (height, edge_pad) = Self::button_length(vertical_space, horizontal_space, 0.1);
        // The arrow is slightly taller than it is wider for artistic reasons.
        let width = height / 1.2;
        (width, height, edge_pad)
    }

    // Paint controls in the basic style.
    #[expect(
        clippy::trivially_copy_pass_by_ref,
        reason = "Widget::paint gets props by ref"
    )]
    fn paint_basic(
        &mut self,
        ctx: &mut PaintCtx<'_>,
        props: &PropertiesRef<'_>,
        scene: &mut Scene,
    ) {
        let color_content = if ctx.is_disabled()
            && let Some(dc) = props.get_defined::<DisabledContentColor>()
        {
            &dc.0
        } else {
            props.get::<ContentColor>()
        };
        let color_backward = props.get::<BackwardColor>();
        let color_forward = props.get::<ForwardColor>();

        let size = ctx.content_box_size();
        let (_, forward, backward) = self.visual_speed();

        let (btn_length, btn_edge_pad) = Self::basic_button_length(size.height, Some(size.width));

        let label_width = self.label_x_end - self.label_x_start;
        let btn_space = ((size.width - label_width) * 0.5).max(0.);

        // Split the horizontal padding to both sides of the button,
        // but constrain the edge padding so we don't go too close nor too far from the edge.
        let btn_x_pad = ((btn_space - btn_length) * 0.5).clamp(btn_edge_pad, btn_length);

        let stroke_width = 0.1 * btn_length;
        let y_center = size.height * 0.5;

        let minus = Line::new(
            Point::new(btn_x_pad, y_center),
            Point::new(btn_x_pad + btn_length, y_center),
        );

        let plus_h = Line::new(
            Point::new(size.width - btn_length - btn_x_pad, y_center),
            Point::new(size.width - btn_x_pad, y_center),
        );
        let plus_v = Line::new(
            Point::new(
                size.width - btn_length * 0.5 - btn_x_pad,
                (size.height - btn_length) * 0.5,
            ),
            Point::new(
                size.width - btn_length * 0.5 - btn_x_pad,
                size.height - (size.height - btn_length) * 0.5,
            ),
        );

        // Only one button can have a special color at the same time.
        // Color choice priority is active > hover > none.
        let (minus_color, plus_color) = if ctx.is_active() && backward {
            (&color_backward.0, &color_content.color)
        } else if ctx.is_active() && forward {
            (&color_content.color, &color_forward.0)
        } else if ctx.is_hovered() && self.hover_backward {
            (&color_backward.0, &color_content.color)
        } else if ctx.is_hovered() && !self.hover_backward {
            (&color_content.color, &color_forward.0)
        } else {
            (&color_content.color, &color_content.color)
        };

        let style = Stroke {
            width: stroke_width,
            start_cap: Cap::Butt,
            end_cap: Cap::Butt,
            ..Default::default()
        };
        scene.stroke(&style, Affine::IDENTITY, minus_color, None, &minus);
        scene.stroke(&style, Affine::IDENTITY, plus_color, None, &plus_h);
        scene.stroke(&style, Affine::IDENTITY, plus_color, None, &plus_v);
    }

    // Paint controls in the flow style.
    #[expect(
        clippy::trivially_copy_pass_by_ref,
        reason = "Widget::paint gets props by ref"
    )]
    fn paint_flow(&mut self, ctx: &mut PaintCtx<'_>, props: &PropertiesRef<'_>, scene: &mut Scene) {
        let color_content = if ctx.is_disabled()
            && let Some(dc) = props.get_defined::<DisabledContentColor>()
        {
            &dc.0
        } else {
            props.get::<ContentColor>()
        };
        let color_backward = props.get::<BackwardColor>();
        let color_forward = props.get::<ForwardColor>();
        let color_heat = props.get::<HeatColor>();

        let size = ctx.content_box_size();
        let (speed, forward, backward) = self.visual_speed();
        let sliding = forward || backward;

        let (arrow_width, arrow_height, arrow_edge_pad) =
            Self::flow_button_length(size.height, Some(size.width));
        let extra_y_pad = (size.height - arrow_height - 2. * arrow_edge_pad) * 0.5;

        // Define the various points that form a hollow-base arrow facing right.
        let arrow_y_center = extra_y_pad + arrow_edge_pad + 0.5 * arrow_height;
        let arrow_tip = Point::new(arrow_width, arrow_y_center);
        let arrow_base_x = 0.575 * arrow_width;
        let arrow_base = Point::new(arrow_base_x, arrow_y_center);
        let arrow_top_shoulder = Point::new(0., extra_y_pad + arrow_edge_pad);
        let arrow_bot_shoulder = Point::new(0., size.height - extra_y_pad - arrow_edge_pad);

        // Construct the arrow shape.
        let mut arrow = BezPath::new();
        arrow.move_to(arrow_top_shoulder);
        arrow.line_to(arrow_tip);
        arrow.line_to(arrow_bot_shoulder);
        arrow.line_to(arrow_base);
        arrow.line_to(arrow_top_shoulder);

        // We want the arrow to move within three full widths of itself,
        // i.e. [AWW] -> [WWA] where A is the arrow and W is empty sapce of the same width.
        let arrow_move_range = 3.;
        // Actual available space for arrow movement depends on the label and our total size.
        let label_width = self.label_x_end - self.label_x_start;
        let arrow_space = ((size.width - label_width) * 0.5 - arrow_edge_pad).max(0.);
        // Base offset is the stationary arrow location,
        // measured from the outer edge of the widget's content-box.
        let arrow_offset_base = arrow_edge_pad + arrow_width * arrow_move_range;
        // Active offset is closer to the edge based on the visual speed factor.
        let arrow_move_active = arrow_move_range - (arrow_move_range - 1.) * speed;
        let arrow_offset_active = arrow_edge_pad + arrow_width * arrow_move_active;
        // Keep the offset within available space bounds if possible,
        // and favor overflowing towards the middle content not the widget bounds.
        // Note that arrow_width > arrow_space is possible, so this can't be f64::clamp.
        let arrow_offset_base = arrow_offset_base.min(arrow_space).max(arrow_width);
        let arrow_offset_active = arrow_offset_active.min(arrow_space).max(arrow_width);
        // Only one arrow can be active at a time.
        let (arrow_offset_forward, arrow_offset_backward) = if forward {
            (arrow_offset_active, arrow_offset_base)
        } else {
            (arrow_offset_base, arrow_offset_active)
        };

        // Construct the affines to get the arrows to their correct locations.
        let arrow_affine_forward = Affine::translate((size.width - arrow_offset_forward, 0.));
        let arrow_affine_backward =
            Affine::reflect((0., 0.), (0., 1.)).then_translate((arrow_offset_backward, 0.).into());

        // Radial gradient located at the tip of the arrow.
        // As the speed grows, more of the arrow will start glowing, spreading from its tip.
        let gradient = sliding.then(|| {
            let radius = (1.5 * speed * arrow_height).max(1.) as f32;
            let gradient = Gradient::new_radial(arrow_tip, radius);
            if backward {
                gradient.with_stops([color_heat.0, color_backward.0])
            } else {
                gradient.with_stops([color_heat.0, color_forward.0])
            }
        });

        // Paint speed lines connecting the label with the arrow.
        if sliding && speed > 0. {
            // The outer lines are thinner and shorter.
            let style1 = Stroke {
                width: 0.075 * arrow_height,
                start_cap: Cap::Butt,
                end_cap: Cap::Butt,
                ..Default::default()
            };
            // The inner lines are thicker and longer.
            let style2 = Stroke {
                width: 0.15 * arrow_height,
                start_cap: Cap::Butt,
                end_cap: Cap::Butt,
                ..Default::default()
            };

            // End the outer lines exactly underneath the arrow shoulder tip.
            let style1_x_end = size.width - arrow_offset_active + arrow_width * 0.11;
            // Keep the outer line length at 80% of the inner lines.
            let style1_x_start = self.label_x_end + (style1_x_end - self.label_x_end) * 0.2;
            // Keep the outer line just barely following the the arrow at the outer edge.
            let style1_y_offset = style1.width * 0.5 + arrow_height * 0.05;

            // End the inner lines underneath the arrow base.
            let style2_x_end = size.width - arrow_offset_active + arrow_base_x;
            // Start from the content.
            let style2_x_start = self.label_x_end;
            // Keep the inner lines following the base of the arrow, with a slight center gap.
            let style2_y_offset = style2.width * 0.5 + arrow_height * 0.06;

            // Top outer line.
            let line1_y = extra_y_pad + arrow_edge_pad + style1_y_offset;
            let line1 = Line::new(
                Point::new(style1_x_start, line1_y),
                Point::new(style1_x_end, line1_y),
            );

            // Top inner line.
            let line2_y = 0.5 * size.height - style2_y_offset;
            let line2 = Line::new(
                Point::new(style2_x_start, line2_y),
                Point::new(style2_x_end, line2_y),
            );

            // Bottom inner line.
            let line3_y = 0.5 * size.height + style2_y_offset;
            let line3 = Line::new(
                Point::new(style2_x_start, line3_y),
                Point::new(style2_x_end, line3_y),
            );

            // Bottom outer line.
            let line4_y = size.height - extra_y_pad - arrow_edge_pad - style1_y_offset;
            let line4 = Line::new(
                Point::new(style1_x_start, line4_y),
                Point::new(style1_x_end, line4_y),
            );

            // Outer lines only go up to 80% opacity based on speed.
            let style1_gradient_max = speed as f32 * 0.8;
            let style1_gradient = Gradient::new_linear((style1_x_start, 0.), (style1_x_end, 0.));
            // Inner lines go up to 100% opacity based on speed.
            let style2_gradient_max = speed as f32;
            let style2_gradient = Gradient::new_linear((style2_x_start, 0.), (style2_x_end, 0.));

            // Create the gradients.
            let style1_gradient = if forward {
                style1_gradient.with_stops([
                    (0., color_forward.0.with_alpha(0.)),
                    (1., color_forward.0.with_alpha(style1_gradient_max)),
                ])
            } else {
                style1_gradient.with_stops([
                    (0., color_backward.0.with_alpha(0.)),
                    (1., color_backward.0.with_alpha(style1_gradient_max)),
                ])
            };
            let style2_gradient = if forward {
                style2_gradient.with_stops([
                    (0., color_forward.0.with_alpha(0.)),
                    (1., color_forward.0.with_alpha(style2_gradient_max)),
                ])
            } else {
                style2_gradient.with_stops([
                    (0., color_backward.0.with_alpha(0.)),
                    (1., color_backward.0.with_alpha(style2_gradient_max)),
                ])
            };

            // The backwards lines need to be reflected and shifted to the other side.
            let lines_affine = if backward {
                Affine::reflect((0., 0.), (0., 1.))
                    .then_translate((self.label_x_end + self.label_x_start, 0.).into())
            } else {
                Affine::IDENTITY
            };

            // Actually paint the lines.
            scene.stroke(&style1, lines_affine, &style1_gradient, None, &line1);
            scene.stroke(&style2, lines_affine, &style2_gradient, None, &line2);
            scene.stroke(&style2, lines_affine, &style2_gradient, None, &line3);
            scene.stroke(&style1, lines_affine, &style1_gradient, None, &line4);
        }

        // Paint the backward facing arrow
        if backward {
            // With a gradient if a slide is in progress
            let gradient = gradient.as_ref().unwrap();
            scene.fill(Fill::NonZero, arrow_affine_backward, gradient, None, &arrow);
        } else {
            // Otherwise with a solid color, potentially showing hover status if not sliding.
            let color = if !sliding && self.hover_backward && ctx.is_hovered() {
                &color_backward.0
            } else {
                &color_content.color
            };
            scene.fill(Fill::NonZero, arrow_affine_backward, color, None, &arrow);
        }

        // Paint the forward facing arrow
        if forward {
            // With a gradient if a slide is in progress
            let gradient = gradient.as_ref().unwrap();
            scene.fill(Fill::NonZero, arrow_affine_forward, gradient, None, &arrow);
        } else {
            // Otherwise with a solid color, potentially showing hover status if not sliding.
            let color = if !sliding && !self.hover_backward && ctx.is_hovered() {
                &color_forward.0
            } else {
                &color_content.color
            };
            scene.fill(Fill::NonZero, arrow_affine_forward, color, None, &arrow);
        }
    }
}

// --- MARK: TESTS
#[cfg(test)]
mod tests {
    use std::fmt::Display;

    use super::*;
    use crate::core::{NewWidget, PropertySet, WidgetOptions, WidgetTag};
    use crate::layout::AsUnit;
    use crate::properties::types::CrossAxisAlignment;
    use crate::properties::{Dimensions, Padding};
    use crate::testing::{TestHarness, assert_render_snapshot};
    use crate::theme::test_property_set;
    use crate::widgets::Flex;

    #[test]
    fn constants() {
        const {
            // It is signed for dev ergonomics, but conceptually it is unsigned.
            // Zero also doesn't make any sense.
            assert!(TICKS_PER_STEP >= 1);

            // Lower gears must end sooner
            assert!(GEAR_1_END < GEAR_2_END);
            assert!(GEAR_1_END_STEPS < GEAR_2_END_STEPS);
        }

        // These constants must match watch the function returns
        assert_eq!(
            GEAR_1_END_STEPS,
            StepInput::<u64>::steps(GEAR_1_END).0 as f64
        );
        assert_eq!(
            GEAR_2_END_STEPS,
            StepInput::<u64>::steps(GEAR_2_END).0 as f64
        );
    }

    #[test]
    fn add_steps() {
        fn assert_add<T: Steppable + Display>(
            base: T,
            step: T,
            snap: Option<T>,
            count: i64,
            expected: T,
        ) {
            let result = base.add_steps(step, count, snap);
            if result != expected {
                panic!(
                    "base {base} \
                    step {step} \
                    count {count} \
                    snap {snap:?} \
                    expected {expected} \
                    got {result}"
                );
            }
        }

        // Signed ints
        assert_add::<i8>(-128, 1, None, 0, -128);
        assert_add::<i8>(-128, 1, None, 1, -127);
        assert_add::<i8>(-128, 1, None, 128, 0);
        assert_add::<i8>(-128, 1, None, 129, 1);
        assert_add::<i8>(-128, 1, None, 254, 126);
        assert_add::<i8>(-128, 1, None, 255, 127);
        assert_add::<i8>(-128, 1, None, 1000, 127);
        assert_add::<i8>(-128, 3, None, 10, -98);
        assert_add::<i8>(-10, 3, None, -10, -40);
        assert_add::<i8>(0, 1, None, 1, 1);
        assert_add::<i8>(0, 1, None, 10, 10);
        assert_add::<i8>(0, 1, None, 200, 127);
        assert_add::<i8>(126, 1, None, 1, 127);
        assert_add::<i8>(126, 1, None, 2, 127);

        assert_add::<i8>(-122, 2, Some(10), -2, -128);
        assert_add::<i8>(-122, 2, Some(10), -1, -120);
        assert_add::<i8>(-50, 2, Some(10), -2, -50);
        assert_add::<i8>(-50, 2, Some(10), -3, -60);
        assert_add::<i8>(50, 2, Some(10), 2, 50);
        assert_add::<i8>(50, 2, Some(10), 3, 60);
        assert_add::<i8>(121, 2, Some(10), 1, 120);
        assert_add::<i8>(121, 2, Some(10), 2, 127);

        // Unsigned ints
        assert_add::<u8>(0, 1, None, 0, 0);
        assert_add::<u8>(0, 1, None, 5, 5);
        assert_add::<u8>(0, 1, None, 255, 255);
        assert_add::<u8>(0, 1, None, 1000, 255);
        assert_add::<u8>(100, 1, None, 100, 200);
        assert_add::<u8>(100, 3, None, 10, 130);
        assert_add::<u8>(254, 3, None, 1, 255);
        assert_add::<u8>(255, 1, None, 1, 255);
        assert_add::<u8>(255, 1, None, -1, 254);
        assert_add::<u8>(255, 1, None, -100, 155);
        assert_add::<u8>(255, 1, None, -1000, 0);

        assert_add::<u8>(2, 2, Some(10), 1, 0);
        assert_add::<u8>(50, 2, Some(10), 2, 50);
        assert_add::<u8>(50, 2, Some(10), 3, 60);
        assert_add::<u8>(251, 2, Some(10), 1, 250);
        assert_add::<u8>(251, 2, Some(10), 2, 255);

        // Floats
        assert_add::<f64>(-100., 0.1, None, 100, -90.);
        assert_add::<f64>(-100., 0.0001, None, 100000, -90.);
        assert_add::<f64>(-100., 0.0001, None, 1000000, 0.);
        assert_add::<f64>(-100., 0.0001, None, 10000000, 900.);

        assert_add::<f64>(-50., 0.1, Some(10.), -20, -50.);
        assert_add::<f64>(-50., 0.1, Some(10.), -60, -60.);
        assert_add::<f64>(50., 0.1, Some(10.), 20, 50.);
        assert_add::<f64>(50., 0.1, Some(10.), 60, 60.);
    }

    #[test]
    fn steps_to() {
        fn assert_count<T: Steppable + Display>(base: T, step: T, target: T, expected: u64) {
            let result = base.steps_to(target, step);
            if result != expected {
                panic!("base {base} step {step} target {target} expected {expected} got {result}");
            }
        }

        // Signed ints
        assert_count::<i8>(-128, 1, -120, 8);
        assert_count::<i8>(-128, 1, 0, 128);
        assert_count::<i8>(-128, 1, 1, 129);
        assert_count::<i8>(-128, 1, 127, 255);
        assert_count::<i8>(-128, 3, -120, 3);
        assert_count::<i8>(0, 1, 127, 127);
        assert_count::<i8>(50, 2, 60, 5);

        // Unsigned ints
        assert_count::<u8>(0, 1, 10, 10);
        assert_count::<u8>(0, 2, 10, 5);
        assert_count::<u8>(5, 2, 10, 3);
        assert_count::<u8>(5, 2, 10, 3);
        assert_count::<u8>(5, 2, 5, 0);
        assert_count::<u8>(0, 1, 255, 255);

        assert_count::<u64>(0, 1, u64::MAX, u64::MAX);

        // Floats
        assert_count::<f64>(-5., 0.1, -3.2, 18);
        assert_count::<f64>(-5., 0.1, 5., 100);
        assert_count::<f64>(5., 0.1, 10., 50);

        assert_count::<f64>(f64::MIN, 1., f64::MAX, u64::MAX);
    }

    #[test]
    fn tick_bounds() {
        #[track_caller]
        fn assert_bounds(base: u64, step: u64, min: u64, max: u64, min_ticks: i64, max_ticks: i64) {
            let si = StepInput::new(base, step, min, max);
            assert!(
                si.min_ticks == min_ticks && si.max_ticks == max_ticks,
                "base: {base}, step: {step}, min: {min}, max: {max} \
                expected ({min_ticks}, {max_ticks}) got ({},{})",
                si.min_ticks,
                si.max_ticks,
            );
        }

        // At the time of writing, this test passes with various TICKS_PER_STEP configurations.
        // When only testing with a single value, certain integer boundary scenarios disappear.
        // Thus, when making significant changes to tick bounds calculations, it would be useful
        // to run this test with various TICKS_PER_STEP configurations, e.g. 1, 3, 7, 10.

        const TPS: i64 = TICKS_PER_STEP;

        // Simple bounds at start, middle, end
        assert_bounds(0, 1, 0, 10, 0, 10 * TPS);
        assert_bounds(5, 1, 0, 10, -5 * TPS, 5 * TPS);
        assert_bounds(10, 1, 0, 10, -10 * TPS, 0);

        // Different step values
        assert_bounds(4, 2, 0, 20, -2 * TPS, 8 * TPS);
        assert_bounds(8, 4, 0, 20, -2 * TPS, 3 * TPS);

        // Max steps representable as a signed tick
        let max_steps = i64::MAX / TPS;

        // Max distance is beyond what the signed ticks can contain, ticks clamped at MAX
        assert_bounds(0, 1, 0, u64::MAX, 0, i64::MAX);
        // Max distance is still beyond signed potential, ticks clamped at MAX
        assert_bounds(0, 1, 0, u64::MAX / TPS as u64, 0, i64::MAX);
        // Max distance is near or at the edge of signed potential, depending on TPS
        assert_bounds(0, 1, 0, max_steps as u64, 0, max_steps * TPS);
        // Max distance is just slightly beyond signed potential, ticks clamped at MAX
        assert_bounds(0, 1, 0, max_steps as u64 + 1, 0, i64::MAX);
        // Max distance is guaranteed below signed potential
        assert_bounds(0, 1, 0, max_steps as u64 - 1, 0, (max_steps - 1) * TPS);

        // Min steps representable as a signed tick
        let min_steps = i64::MIN / TPS;
        let min_steps_neg = min_steps.wrapping_neg() as u64;

        // Min distance is beyond what the signed ticks can contain, ticks clamped at MIN
        assert_bounds(u64::MAX, 1, 0, u64::MAX, i64::MIN, 0);
        // Min distance is still beyond signed potential, ticks clamped at MIN
        assert_bounds(
            u64::MAX / TPS as u64,
            1,
            0,
            u64::MAX / TPS as u64,
            i64::MIN,
            0,
        );
        // Min distance is near or at the edge of signed potential, depending on TPS
        assert_bounds(min_steps_neg, 1, 0, min_steps_neg, min_steps * TPS, 0);
        // Min distance is just slightly beyond signed potential, ticks clamped at MIN
        assert_bounds(min_steps_neg + 1, 1, 0, min_steps_neg + 1, i64::MIN, 0);
        // Min distance is guaranteed below signed potential
        assert_bounds(
            min_steps_neg - 1,
            1,
            0,
            min_steps_neg - 1,
            (min_steps + 1) * TPS,
            0,
        );
    }

    #[test]
    fn steps_and_distance() {
        fn assert_sync(steps: f64) {
            // Calculate the distance covered by these steps
            let distance = StepInput::<u64>::distance(steps);

            // Calculate expected steps and drift from the input steps
            let direction = steps.signum();
            let steps = steps.abs();

            let expected_steps = (steps.round() * direction) as i64;
            let expected_drift = if steps > GEAR_2_END_STEPS {
                (steps - steps.round()) / GEAR_3_RATIO
            } else if steps > GEAR_1_END_STEPS {
                (steps - steps.round()) / GEAR_2_RATIO
            } else {
                (steps - steps.round()) / GEAR_1_RATIO
            } * direction;

            // Make sure the steps function meets expectations
            let result = StepInput::<u64>::steps(distance);

            assert!(
                result.0 == expected_steps && (result.1 - expected_drift).abs() <= 1e-10,
                "assert_sync({}) => \
				for {distance} expected ({expected_steps}, {expected_drift}), got {result:?}",
                steps * direction,
            );
        }

        let test_steps = [
            0.0,
            0.01,
            0.1,
            0.4,
            0.6,
            0.99,
            1.0,
            1.1,
            5.01,
            GEAR_1_END_STEPS - 0.1,
            GEAR_1_END_STEPS - 0.01,
            GEAR_1_END_STEPS,
            GEAR_1_END_STEPS + 0.01,
            GEAR_1_END_STEPS + 0.1,
            GEAR_1_END_STEPS + 0.4,
            GEAR_1_END_STEPS + 0.6,
            GEAR_1_END_STEPS + 0.99,
            GEAR_1_END_STEPS + 1.0,
            GEAR_1_END_STEPS + 1.1,
            GEAR_1_END_STEPS + 5.01,
            GEAR_2_END_STEPS - 0.1,
            GEAR_2_END_STEPS - 0.01,
            GEAR_2_END_STEPS,
            GEAR_2_END_STEPS + 0.01,
            GEAR_2_END_STEPS + 0.1,
            GEAR_2_END_STEPS + 0.4,
            GEAR_2_END_STEPS + 0.6,
            GEAR_2_END_STEPS + 0.99,
            GEAR_2_END_STEPS + 1.0,
            GEAR_2_END_STEPS + 1.1,
            GEAR_2_END_STEPS + 5.01,
        ];

        for steps in test_steps {
            assert_sync(steps);
            assert_sync(-steps);
        }
    }

    #[test]
    fn update_ticks() {
        #[derive(Debug, Copy, Clone)]
        struct InitialState {
            value: u64,
            step: u64,
            min: u64,
            max: u64,
            wrap: bool,
        }

        #[track_caller]
        fn assert_update(
            initial_state: InitialState,
            delta: i64,
            delta_unused: i64,
            new_value: Option<u64>,
            new_ticks: i64,
        ) {
            let mut si = StepInput::new(
                initial_state.value,
                initial_state.step,
                initial_state.min,
                initial_state.max,
            );
            si.wrap = initial_state.wrap;
            let result = si.update_ticks(delta, false);
            let result_new_value = result.1.then_some(si.value);

            assert!(
                result.0 == delta_unused && result_new_value == new_value && si.ticks == new_ticks,
                "{initial_state:?}, delta: {delta}, \
                expected (got) \
                delta_unused: {delta_unused} ({}), \
                new_value: {new_value:?} ({:?}), \
                new_ticks: {new_ticks} ({})",
                result.0,
                result_new_value,
                si.ticks,
            );
        }

        const TPS: i64 = TICKS_PER_STEP;

        let mut is = InitialState {
            value: 0,
            step: 5,
            min: 0,
            max: 50,
            wrap: false,
        };

        // Test negative unused delta overflow protection
        assert_update(is, i64::MIN, i64::MIN, None, 0);
        // One step back
        assert_update(is, -TPS, -TPS, None, 0);
        // Single tick back
        if TPS >= 3 {
            assert_update(is, -1, -1, None, 0);
        }
        // No real change at all
        assert_update(is, 0, 0, None, 0);
        // No changed value due to rounding down
        if TPS >= 2 {
            assert_update(is, TPS / 2 - 1, 0, None, TPS / 2 - 1);
        }
        // One step forward due to rounding ticks up
        assert_update(is, TPS / 2 + 1, 0, Some(5), TPS / 2 + 1);
        // One step forward due to rounding ticks up
        if TPS >= 2 {
            assert_update(is, TPS - 1, 0, Some(5), TPS - 1);
        }
        // One step forward
        assert_update(is, TPS, 0, Some(5), TPS);
        // One step forward due to rounding ticks down
        if TPS >= 3 {
            assert_update(is, TPS + 1, 0, Some(5), TPS + 1);
        }
        // Two steps forward due to rounding ticks down
        if TPS >= 2 {
            assert_update(is, TPS * 2 - 1, 0, Some(10), TPS * 2 - 1);
        }
        // Two steps forward
        assert_update(is, TPS * 2, 0, Some(10), TPS * 2);
        // Ten steps forward, to the max edge
        assert_update(is, TPS * 10, 0, Some(50), TPS * 10);
        // Ten steps + one tick forward, clamped at max edge
        assert_update(is, TPS * 10 + 1, 1, Some(50), TPS * 10);
        // Elevent steps forward, clamped at max edge
        assert_update(is, TPS * 11, TPS, Some(50), TPS * 10);

        // Test difficult int boundaries.
        // We set the value to the midpoint, so that the distance
        // to min is the signed minimum and that the distance
        // to max is the signed maximum.
        let mid = i64::MAX as u64 + 1;
        is = InitialState {
            value: mid,
            step: 1,
            min: u64::MIN,
            max: u64::MAX,
            wrap: false,
        };

        // Expected value might be off by one due to rounding
        fn rounding(ticks: i64) -> u64 {
            let remainder = (ticks % TPS).unsigned_abs();
            if remainder > 0 && remainder * 2 >= TPS as u64 {
                1
            } else {
                0
            }
        }

        let min_effect = (i64::MIN / TPS).unsigned_abs();

        // Minimum possible distance
        assert_update(is, i64::MIN, 0, Some(mid - min_effect), i64::MIN);
        // One step less than minimum
        assert_update(
            is,
            i64::MIN + TPS,
            0,
            Some(mid - min_effect + 1 - rounding(i64::MIN + TPS)),
            i64::MIN + TPS,
        );
        // Two steps less than minimum
        assert_update(
            is,
            i64::MIN + 2 * TPS,
            0,
            Some(mid - min_effect + 2 - rounding(i64::MIN + 2 * TPS)),
            i64::MIN + 2 * TPS,
        );

        // One step back from mid
        assert_update(is, -TPS, 0, Some(mid - 1), -TPS);
        // One step forward from mid
        assert_update(is, TPS, 0, Some(mid + 1), TPS);

        let max_effect = (i64::MAX / TPS) as u64;

        // Two steps less than maximum
        assert_update(
            is,
            i64::MAX - 2 * TPS,
            0,
            Some(mid + max_effect - 2 + rounding(i64::MAX - 2 * TPS)),
            i64::MAX - 2 * TPS,
        );
        // One step less than maximum
        assert_update(
            is,
            i64::MAX - TPS,
            0,
            Some(mid + max_effect - 1 + rounding(i64::MAX - TPS)),
            i64::MAX - TPS,
        );
        // Maximum possible distance
        assert_update(is, i64::MAX, 0, Some(mid + max_effect), i64::MAX);

        // Test wrapping forward
        is = InitialState {
            value: u64::MAX - 1,
            step: 1,
            min: u64::MIN,
            max: u64::MAX,
            wrap: true,
        };

        // Step to the max edge
        assert_update(is, TPS, 0, Some(u64::MAX), TPS);
        // Step to the max edge plus one tick
        if TPS >= 2 {
            assert_update(is, TPS + 1, 1, Some(u64::MAX), TPS);
        }
        // Wrap around to min edge
        assert_update(is, 2 * TPS, 0, Some(0), 0);
        // Wrap around to min edge plus another step
        assert_update(is, 3 * TPS, 0, Some(1), TPS);

        // Test wrapping backwards
        is = InitialState {
            value: 1,
            step: 1,
            min: u64::MIN,
            max: u64::MAX,
            wrap: true,
        };

        // Step to the min edge
        assert_update(is, -TPS, 0, Some(0), -TPS);
        // Step to the min edge plus one tick
        if TPS >= 2 {
            assert_update(is, -TPS - 1, -1, Some(0), -TPS);
        }
        // Wrap around to max edge
        assert_update(is, -2 * TPS, 0, Some(u64::MAX), 0);
        // Wrap around to max edge plus another step
        assert_update(is, -3 * TPS, 0, Some(u64::MAX - 1), -TPS);

        // Test multi-wrap
        is = InitialState {
            value: 10,
            step: 1,
            min: 0,
            max: 20,
            wrap: true,
        };

        // Wrap around to min edge
        assert_update(is, 11 * TPS, 0, Some(0), 0);
        // Wrap around to min edge 2x
        assert_update(is, 32 * TPS, 0, Some(0), 0);
        // Wrap around to min edge 3x + 2 steps
        assert_update(is, 55 * TPS, 0, Some(2), 2 * TPS);

        // Wrap around to max edge
        assert_update(is, -11 * TPS, 0, Some(20), 0);
        // Wrap around to max edge 2x
        assert_update(is, -32 * TPS, 0, Some(20), 0);
        // Wrap around to max edge 3x + 2 steps
        assert_update(is, -55 * TPS, 0, Some(18), -2 * TPS);
    }

    #[test]
    fn basics() {
        let si = |base, props| StepInput::new(base, 1, 0, usize::MAX).with_props(props);

        let root = Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .with_fixed(si(0, (StepInputStyle::Basic, Dimensions::MIN)))
            .with_fixed(si(0, (StepInputStyle::Flow, Dimensions::MIN)))
            .with_fixed(si(0, (StepInputStyle::Basic, Dimensions::MAX)))
            .with_fixed(si(0, (StepInputStyle::Flow, Dimensions::MAX)))
            .with_fixed(si(0, (StepInputStyle::Basic, Dimensions::width(100.px()))))
            .with_fixed(si(0, (StepInputStyle::Flow, Dimensions::width(100.px()))))
            .with_fixed(si(500, (StepInputStyle::Basic, Dimensions::MIN)))
            .with_fixed(si(500, (StepInputStyle::Flow, Dimensions::MIN)))
            .with_fixed(si(500, (StepInputStyle::Basic, Dimensions::MAX)))
            .with_fixed(si(500, (StepInputStyle::Flow, Dimensions::MAX)))
            .with_fixed(si(
                500,
                (StepInputStyle::Basic, Dimensions::width(100.px())),
            ))
            .with_fixed(si(500, (StepInputStyle::Flow, Dimensions::width(100.px()))))
            .with_props(Padding::all(10.));

        let window_size = Size::new(150.0, 525.0);
        let mut harness = TestHarness::create_with_size(test_property_set(), root, window_size);

        assert_render_snapshot!(harness, "step_input_basics");
    }

    #[test]
    fn speed_lines() {
        let si = |tag| {
            NewWidget::new_with(
                StepInput::new(5000, 1, 0, usize::MAX),
                Some(tag),
                WidgetOptions::default(),
                (StepInputStyle::Flow, Dimensions::fixed(250.px(), 85.px())),
            )
        };

        let tag_backward = WidgetTag::unique();
        let tag_forward = WidgetTag::unique();

        let lines_backward = si(tag_backward);
        let lines_forward = si(tag_forward);

        let root = Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .with_fixed(lines_backward)
            .with_fixed(lines_forward)
            .with_props(Padding::all(10.));

        let window_size = Size::new(270.0, 200.0);
        let mut harness = TestHarness::create_with_size(test_property_set(), root, window_size);

        harness.edit_widget(tag_backward, |mut widget| {
            widget.widget.drag_start = Some(Point::new(10000., 0.));
            widget.widget.slide_last = Some(Point::ORIGIN);
            widget.ctx.request_paint_only();
        });

        harness.edit_widget(tag_forward, |mut widget| {
            widget.widget.drag_start = Some(Point::new(10000., 0.));
            widget.widget.slide_last = Some(Point::new(20000., 0.));
            widget.ctx.request_paint_only();
        });

        assert_render_snapshot!(harness, "step_input_speed_lines");
    }

    #[test]
    fn awkward_layout() {
        let basic = |base, props: PropertySet| {
            let props = props.with(StepInputStyle::Basic);
            StepInput::new(base, 1, 0, usize::MAX).with_props(props)
        };
        let flow = |base, props: PropertySet| {
            let props = props.with(StepInputStyle::Flow);
            StepInput::new(base, 1, 0, usize::MAX).with_props(props)
        };

        let root = Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Start)
            // Below MinContent size to test that controls don't overflow out of the widget.
            .with_fixed(basic(1234, (Dimensions::fixed(50.px(), 30.px())).into()))
            .with_fixed(flow(1234, (Dimensions::fixed(50.px(), 30.px())).into()))
            // Zero padding to test that controls don't touch borders.
            .with_fixed(basic(
                1234,
                (Padding::ZERO, Dimensions::fixed(100.px(), 18.px())).into(),
            ))
            .with_fixed(flow(
                1234,
                (Padding::ZERO, Dimensions::fixed(100.px(), 18.px())).into(),
            ))
            // Extra wide widget to test that controls remain reasonably sized.
            .with_fixed(basic(1234, (Dimensions::fixed(300.px(), 30.px())).into()))
            .with_fixed(flow(1234, (Dimensions::fixed(300.px(), 30.px())).into()))
            // Extra high widget to test that controls remain reasonably sized.
            .with_fixed(basic(1234, (Dimensions::fixed(100.px(), 200.px())).into()))
            .with_fixed(flow(1234, (Dimensions::fixed(100.px(), 200.px())).into()))
            .with_props(Padding::all(10.));

        let window_size = Size::new(320.0, 650.0);
        let mut harness = TestHarness::create_with_size(test_property_set(), root, window_size);

        assert_render_snapshot!(harness, "step_input_awkward_layout");
    }
}
