use std::{collections::HashMap, mem::Discriminant};

type StyleProperty<Brush: parley::Brush> = parley::StyleProperty<'static, Brush>;

/// A set of Parley styles.
#[derive(Clone, Debug)]
pub struct StyleSet<Brush: parley::Brush>(
    HashMap<Discriminant<StyleProperty<Brush>>, StyleProperty<Brush>>,
);

impl<Brush: parley::Brush> StyleSet<Brush> {
    pub fn new(font_size: f32) -> Self {
        let mut this = Self(Default::default());
        this.insert(StyleProperty::FontSize(font_size));
        this
    }

    pub fn insert(&mut self, style: StyleProperty<Brush>) -> Option<StyleProperty<Brush>> {
        let discriminant = std::mem::discriminant(&style);
        self.0.insert(discriminant, style)
    }

    pub fn retain(&mut self, mut f: impl FnMut(&StyleProperty<Brush>) -> bool) {
        self.0.retain(|_, v| f(v));
    }

    pub fn remove(
        &mut self,
        property: Discriminant<StyleProperty<Brush>>,
    ) -> Option<StyleProperty<Brush>> {
        self.0.remove(&property)
    }

    pub fn inner(&self) -> &HashMap<Discriminant<StyleProperty<Brush>>, StyleProperty<Brush>> {
        &self.0
    }
}
