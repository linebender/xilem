// Copyright 2023 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

type CowStr = std::borrow::Cow<'static, str>;

/// Representation of an attribute value.
///
/// This type is used as optimization, to avoid allocations, as it's copied around a lot
#[derive(PartialEq, Clone, Debug, PartialOrd)]
pub enum AttributeValue {
    True, // for the boolean true, this serializes to an empty string (e.g. for <input checked>)
    I16(i16),
    U16(u16),
    I32(i32),
    U32(u32),
    Usize(usize),
    F32(f32),
    F64(f64),
    String(CowStr),
}

impl AttributeValue {
    pub fn serialize(&self) -> CowStr {
        match self {
            AttributeValue::True => "".into(), // empty string is equivalent to a true set attribute
            AttributeValue::I16(n) => n.to_string().into(),
            AttributeValue::U16(n) => n.to_string().into(),
            AttributeValue::I32(n) => n.to_string().into(),
            AttributeValue::U32(n) => n.to_string().into(),
            AttributeValue::Usize(n) => n.to_string().into(),
            AttributeValue::F32(n) => n.to_string().into(),
            AttributeValue::F64(n) => n.to_string().into(),
            AttributeValue::String(s) => s.clone(),
        }
    }
}

/// Types implementing this trait can be used as value in e.g. [`Element::attr`](`crate::interfaces::Element::attr`)
pub trait IntoAttributeValue: Sized {
    fn into_attr_value(self) -> Option<AttributeValue>;
}

impl<T: IntoAttributeValue> IntoAttributeValue for Option<T> {
    fn into_attr_value(self) -> Option<AttributeValue> {
        if let Some(value) = self {
            T::into_attr_value(value)
        } else {
            None
        }
    }
}

impl IntoAttributeValue for bool {
    fn into_attr_value(self) -> Option<AttributeValue> {
        self.then_some(AttributeValue::True)
    }
}

impl IntoAttributeValue for AttributeValue {
    fn into_attr_value(self) -> Option<AttributeValue> {
        Some(self)
    }
}

impl IntoAttributeValue for i16 {
    fn into_attr_value(self) -> Option<AttributeValue> {
        Some(AttributeValue::I16(self))
    }
}

impl IntoAttributeValue for u16 {
    fn into_attr_value(self) -> Option<AttributeValue> {
        Some(AttributeValue::U16(self))
    }
}

impl IntoAttributeValue for i32 {
    fn into_attr_value(self) -> Option<AttributeValue> {
        Some(AttributeValue::I32(self))
    }
}

impl IntoAttributeValue for u32 {
    fn into_attr_value(self) -> Option<AttributeValue> {
        Some(AttributeValue::U32(self))
    }
}

impl IntoAttributeValue for usize {
    fn into_attr_value(self) -> Option<AttributeValue> {
        Some(AttributeValue::Usize(self))
    }
}

impl IntoAttributeValue for f32 {
    fn into_attr_value(self) -> Option<AttributeValue> {
        Some(AttributeValue::F32(self))
    }
}

impl IntoAttributeValue for f64 {
    fn into_attr_value(self) -> Option<AttributeValue> {
        Some(AttributeValue::F64(self))
    }
}

impl IntoAttributeValue for String {
    fn into_attr_value(self) -> Option<AttributeValue> {
        Some(AttributeValue::String(self.into()))
    }
}

impl IntoAttributeValue for CowStr {
    fn into_attr_value(self) -> Option<AttributeValue> {
        Some(AttributeValue::String(self))
    }
}

impl IntoAttributeValue for &'static str {
    fn into_attr_value(self) -> Option<AttributeValue> {
        Some(AttributeValue::String(self.into()))
    }
}
