use std::ops::Range;

use super::Selectable;

/// Text which can be edited
pub trait EditableText: Selectable {
    /// Replace range with new text.
    /// Can panic if supplied an invalid range.
    // TODO: make this generic over Self
    fn edit(&mut self, range: Range<usize>, new: impl Into<String>);
    /// Create a value of this struct
    fn from_str(s: &str) -> Self;
}

impl EditableText for String {
    fn edit(&mut self, range: Range<usize>, new: impl Into<String>) {
        self.replace_range(range, &new.into());
    }
    fn from_str(s: &str) -> Self {
        s.to_string()
    }
}

// TODO: What advantage does this actually have?
// impl EditableText for Arc<String> {
//     fn edit(&mut self, range: Range<usize>, new: impl Into<String>) {
//         let new = new.into();
//         if !range.is_empty() || !new.is_empty() {
//             Arc::make_mut(self).edit(range, new)
//         }
//     }
//     fn from_str(s: &str) -> Self {
//         Arc::new(s.to_owned())
//     }
// }

#[cfg(test)]
mod tests {
    use super::EditableText;

    // #[test]
    // fn arcstring_empty_edit() {
    //     let a = Arc::new("hello".to_owned());
    //     let mut b = a.clone();
    //     b.edit(5..5, "");
    //     assert!(Arc::ptr_eq(&a, &b));
    // }

    #[test]
    fn replace() {
        let mut a = String::from("hello world");
        a.edit(1..9, "era");
        assert_eq!("herald", a);
    }
}
