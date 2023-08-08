use std::borrow::Cow;

use crate::vecmap::VecMap;

type CowStr = Cow<'static, str>;

#[derive(PartialEq, Debug)]
pub enum AttributeValue {
    Null,
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
            // AttributeValue::Null shouldn't be serialized within attribute serialization/diffing
            // as null values should never show up in the attributes map, but for completeness it's serialized to an empty string
            AttributeValue::Null => "".into(),
        }
    }
}

// A few convenient From implementations for less boilerplate when setting attributes

impl<T: Into<AttributeValue>> From<Option<T>> for AttributeValue {
    fn from(value: Option<T>) -> Self {
        if let Some(value) = value {
            value.into()
        } else {
            AttributeValue::Null
        }
    }
}

impl From<bool> for AttributeValue {
    fn from(value: bool) -> Self {
        if value {
            AttributeValue::True
        } else {
            AttributeValue::Null
        }
    }
}

impl From<u32> for AttributeValue {
    fn from(value: u32) -> Self {
        AttributeValue::U32(value)
    }
}

impl From<i32> for AttributeValue {
    fn from(value: i32) -> Self {
        AttributeValue::I32(value)
    }
}

impl From<f32> for AttributeValue {
    fn from(value: f32) -> Self {
        AttributeValue::F32(value)
    }
}

impl From<f64> for AttributeValue {
    fn from(value: f64) -> Self {
        AttributeValue::F64(value)
    }
}

impl From<String> for AttributeValue {
    fn from(value: String) -> Self {
        AttributeValue::String(value.into())
    }
}

impl From<CowStr> for AttributeValue {
    fn from(value: CowStr) -> Self {
        AttributeValue::String(value)
    }
}

impl From<&'static str> for AttributeValue {
    fn from(value: &'static str) -> Self {
        AttributeValue::String(value.into())
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
    pub fn insert(&mut self, name: impl Into<CowStr>, value: impl Into<AttributeValue>) {
        let value = value.into();
        // This is a simple optimization in case this is the first attribute inserted to the map (saves an allocation for the Vec)
        if matches!(value, AttributeValue::Null) {
            self.0.remove(&name.into());
        } else {
            self.0.insert(name.into(), value);
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (&CowStr, &AttributeValue)> {
        self.0.iter()
    }
}
