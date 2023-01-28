use std::ops::Range;
use vello::kurbo::{Point, Rect, Size, Vec2};
use crate::widget::BoxConstraints;

/// An axis in visual space.
///
/// Most often used by widgets to describe
/// the direction in which they grow as their number of children increases.
/// Has some methods for manipulating geometry with respect to the axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    /// The x axis
    Horizontal,
    /// The y axis
    Vertical,
}

impl Axis {
    pub fn cross(self) -> Self {
        match self {
            Axis::Horizontal => Axis::Vertical,
            Axis::Vertical => Axis::Horizontal,
        }
    }

    pub fn major<T: Dim2>(self, value: T) -> T::Scalar {
        match self {
            Axis::Horizontal => value.x(),
            Axis::Vertical => value.y(),
        }
    }

    pub fn minor<T: Dim2>(self, value: T) -> T::Scalar {
        self.cross().major(value)
    }

    pub fn with_major<T: Dim2>(self, value: T, major: T::Scalar) -> T {
        match self {
            Axis::Horizontal => T::new(major, value.y()),
            Axis::Vertical => T::new(value.x(), major),
        }
    }

    pub fn with_minor<T: Dim2>(self, value: T, minor: T::Scalar) -> T {
        self.cross().with_major(value, minor)
    }

    pub fn map_major<T: Dim2 + Copy, F: FnOnce(T::Scalar) -> T::Scalar>(self, value: T, map: F) -> T {
        match self {
            Axis::Horizontal => T::new(map(value.x()), value.y()),
            Axis::Vertical => T::new(value.x(), map(value.y())),
        }
    }

    pub fn map_minor<T: Dim2 + Copy, F: FnOnce(T::Scalar) -> T::Scalar>(self, value: T, map: F) -> T {
        self.cross().map_major(value, map)
    }

    pub fn pack<T: Dim2>(self, major: T::Scalar, minor: T::Scalar) -> T {
        match self {
            Axis::Horizontal => T::new(major, minor),
            Axis::Vertical => T::new(minor, major),
        }
    }
}

pub trait Dim2 {
    type Scalar;

    fn new(x: Self::Scalar, y: Self::Scalar) -> Self;

    fn x(self) -> Self::Scalar;

    fn y(self) -> Self::Scalar;
}

impl Dim2 for Point {
    type Scalar = f64;

    fn new(x: Self::Scalar, y: Self::Scalar) -> Self {
        Point::new(x, y)
    }

    fn x(self) -> Self::Scalar {
        self.x
    }

    fn y(self) -> Self::Scalar {
        self.y
    }
}

impl Dim2 for Vec2 {
    type Scalar = f64;

    fn new(x: Self::Scalar, y: Self::Scalar) -> Self {
        Vec2::new(x, y)
    }

    fn x(self) -> Self::Scalar {
        self.x
    }

    fn y(self) -> Self::Scalar {
        self.y
    }
}

impl Dim2 for Size {
    type Scalar = f64;

    fn new(x: Self::Scalar, y: Self::Scalar) -> Self {
        Size::new(x, y)
    }

    fn x(self) -> Self::Scalar {
        self.x
    }

    fn y(self) -> Self::Scalar {
        self.y
    }
}

struct Span {
    low: f64,
    high: f64,
}

impl Dim2 for Rect {
    type Scalar = Span;

    fn new(x: Self::Scalar, y: Self::Scalar) -> Self {
        Rect::new(x.low, y.low, x.high, y.high)
    }

    fn x(self) -> Self::Scalar {
        Span {low: self.x0, high: self.x1}
    }

    fn y(self) -> Self::Scalar {
        Span {low: self.y0, high: self.y1}
    }
}

impl Dim2 for BoxConstraints {
    type Scalar = Range<f64>;

    fn new(x: Self::Scalar, y: Self::Scalar) -> Self {
        BoxConstraints::new(Size::new(x.start, y.start), Size::new(x.end, y.end))
    }

    fn x(self) -> Self::Scalar {
        self.min().width..self.max().width
    }

    fn y(self) -> Self::Scalar {
        self.min().height..self.max().height
    }
}