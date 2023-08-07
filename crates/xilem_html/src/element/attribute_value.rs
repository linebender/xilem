use std::collections::BTreeSet;

type CowStr = std::borrow::Cow<'static, str>;

// TODO not sure how useful an extra enum for attribute keys is (comparison is probably a little bit faster...)
// #[derive(PartialEq, Eq)]
// enum AttrKey {
//     Width,
//     Height,
//     Class,
//     Untyped(Box<Cow<'static, str>>),
// }

#[derive(PartialEq, Debug)]
pub enum AttributeValue {
    True, // for the boolean true, this serializes to an empty string (e.g. for <input checked>)
    I32(i32),
    U32(u32),
    F32(f32),
    F64(f64),
    String(CowStr),
    Classes(BTreeSet<CowStr>),
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
            // TODO maybe use Vec as backend (should probably be more performant for few classes, which seems to be the average case)
            AttributeValue::Classes(set) => set
                .iter()
                .fold(String::new(), |mut acc, s| {
                    if !acc.is_empty() {
                        acc += " ";
                    }
                    if !s.is_empty() {
                        acc += s;
                    }
                    acc
                })
                .into(),
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
