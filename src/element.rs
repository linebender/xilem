use std::any::Any;
use crate::widget::ChangeFlags;

pub trait Element: 'static {
    fn downcast_mut<T: 'static>(&mut self) -> Option<&mut T>;

    fn mark(&mut self, flags: ChangeFlags) -> ChangeFlags;
}