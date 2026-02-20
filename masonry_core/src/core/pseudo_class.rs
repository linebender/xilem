// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Pseudo-class state tracking for widgets.
//!
//! [`PseudoSet`] is a compact bitfield that tracks interaction and widget-defined
//! pseudo states (hovered, active, focused, disabled, toggled, etc.).
//! The framework maintains built-in pseudo states automatically; widgets can
//! define additional states starting from bit 5.

use std::fmt;
use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not};

/// A pseudo-class identifier, representing a single bit index (0..63) in a [`PseudoSet`].
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct PseudoId(pub u8);

impl PseudoId {
    /// Built-in: the widget is hovered by a pointer.
    pub const HOVER: Self = Self(0);
    /// Built-in: the widget is active (e.g. pointer captured).
    pub const ACTIVE: Self = Self(1);
    /// Built-in: the widget has text focus.
    pub const FOCUS: Self = Self(2);
    /// Built-in: this widget or a descendant has focus.
    pub const FOCUS_WITHIN: Self = Self(3);
    /// Built-in: the widget is disabled.
    pub const DISABLED: Self = Self(4);
}

impl fmt::Debug for PseudoId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            0 => write!(f, "PseudoId::HOVER"),
            1 => write!(f, "PseudoId::ACTIVE"),
            2 => write!(f, "PseudoId::FOCUS"),
            3 => write!(f, "PseudoId::FOCUS_WITHIN"),
            4 => write!(f, "PseudoId::DISABLED"),
            n => write!(f, "PseudoId({n})"),
        }
    }
}

/// A bitfield of pseudo-class states, stored as a `u64`.
///
/// Each bit corresponds to a [`PseudoId`]. Bits 0..4 are reserved for
/// framework-managed states; bits 5..63 are available for widget or
/// application-defined pseudo states.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct PseudoSet(pub u64);

impl PseudoSet {
    /// The empty set — no pseudo states active.
    pub const EMPTY: Self = Self(0);

    /// Returns `true` if the given pseudo state is set.
    pub fn contains(self, id: PseudoId) -> bool {
        debug_assert!(id.0 < 64, "PseudoId out of range");
        self.0 & (1_u64 << id.0) != 0
    }

    /// Sets or clears a pseudo state.
    pub fn set(&mut self, id: PseudoId, value: bool) {
        debug_assert!(id.0 < 64, "PseudoId out of range");
        if value {
            self.0 |= 1_u64 << id.0;
        } else {
            self.0 &= !(1_u64 << id.0);
        }
    }

    /// Returns `true` if no pseudo states are set.
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }
}

impl fmt::Debug for PseudoSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PseudoSet(0b{:b})", self.0)
    }
}

impl BitOr for PseudoSet {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for PseudoSet {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for PseudoSet {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for PseudoSet {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl Not for PseudoSet {
    type Output = Self;
    fn not(self) -> Self {
        Self(!self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_and_contains() {
        let mut set = PseudoSet::EMPTY;
        assert!(!set.contains(PseudoId::HOVER));

        set.set(PseudoId::HOVER, true);
        assert!(set.contains(PseudoId::HOVER));
        assert!(!set.contains(PseudoId::ACTIVE));

        set.set(PseudoId::HOVER, false);
        assert!(!set.contains(PseudoId::HOVER));
    }

    #[test]
    fn bitwise_ops() {
        let mut a = PseudoSet::EMPTY;
        a.set(PseudoId::HOVER, true);

        let mut b = PseudoSet::EMPTY;
        b.set(PseudoId::ACTIVE, true);

        let union = a | b;
        assert!(union.contains(PseudoId::HOVER));
        assert!(union.contains(PseudoId::ACTIVE));

        let intersection = a & b;
        assert!(intersection.is_empty());
    }
}
