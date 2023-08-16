use std::borrow::Cow;

use crate::vecmap::VecMap;

type CowStr = Cow<'static, str>;

#[derive(PartialEq, Debug)]
pub enum AttributeValue {
    True, // for the boolean true, this serializes to an empty string (e.g. for <input checked>)
    I32(i32),
    U32(u32),
    F32(f32),
    F64(f64),
    String(CowStr),
}

impl AttributeValue {
    pub fn serialize(&self) -> CowStr {
        match self {
            AttributeValue::True => "".into(), // empty string is equivalent to a true set attribute
            AttributeValue::I32(n) => n.to_string().into(),
            AttributeValue::U32(n) => n.to_string().into(),
            AttributeValue::F32(n) => n.to_string().into(),
            AttributeValue::F64(n) => n.to_string().into(),
            AttributeValue::String(s) => s.clone(),
        }
    }
}

pub trait IntoAttributeValue: Sized {
    fn into_attribute_value(self) -> Option<AttributeValue>;
}

impl<T: IntoAttributeValue> IntoAttributeValue for Option<T> {
    fn into_attribute_value(self) -> Option<AttributeValue> {
        if let Some(value) = self {
            T::into_attribute_value(value)
        } else {
            None
        }
    }
}

impl IntoAttributeValue for bool {
    fn into_attribute_value(self) -> Option<AttributeValue> {
        self.then_some(AttributeValue::True)
    }
}

impl IntoAttributeValue for AttributeValue {
    fn into_attribute_value(self) -> Option<AttributeValue> {
        Some(self)
    }
}

impl IntoAttributeValue for u32 {
    fn into_attribute_value(self) -> Option<AttributeValue> {
        Some(AttributeValue::U32(self))
    }
}

impl IntoAttributeValue for i32 {
    fn into_attribute_value(self) -> Option<AttributeValue> {
        Some(AttributeValue::I32(self))
    }
}

impl IntoAttributeValue for f32 {
    fn into_attribute_value(self) -> Option<AttributeValue> {
        Some(AttributeValue::F32(self))
    }
}

impl IntoAttributeValue for f64 {
    fn into_attribute_value(self) -> Option<AttributeValue> {
        Some(AttributeValue::F64(self))
    }
}

impl IntoAttributeValue for String {
    fn into_attribute_value(self) -> Option<AttributeValue> {
        Some(AttributeValue::String(self.into()))
    }
}

impl IntoAttributeValue for CowStr {
    fn into_attribute_value(self) -> Option<AttributeValue> {
        Some(AttributeValue::String(self))
    }
}

impl IntoAttributeValue for &'static str {
    fn into_attribute_value(self) -> Option<AttributeValue> {
        Some(AttributeValue::String(self.into()))
    }
}

#[derive(Default)]
pub struct Attributes(VecMap<CowStr, AttributeValue>);

impl<'a> IntoIterator for &'a Attributes {
    type Item = (&'a CowStr, &'a AttributeValue);

    type IntoIter = <&'a VecMap<CowStr, AttributeValue> as std::iter::IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl Attributes {
    // TODO return the previous attribute as an Option?
    pub fn insert(&mut self, name: impl Into<CowStr>, value: impl IntoAttributeValue) {
        let value = value.into_attribute_value();
        // This is a simple optimization in case this is the first attribute inserted to the map (saves an allocation for the Vec)
        if let Some(value) = value.into_attribute_value() {
            self.0.insert(name.into(), value);
        } else {
            self.0.remove(&name.into());
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (&CowStr, &AttributeValue)> {
        self.0.iter()
    }
}
