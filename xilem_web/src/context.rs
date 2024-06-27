use crate::{
    app::{AppMessage, AppRunner},
    core::{ViewId, ViewPathTracker},
    Message,
};

type IdPath = Vec<ViewId>;

pub struct MessageThunk {
    id_path: IdPath,
    app_ref: Box<dyn AppRunner>,
}

impl MessageThunk {
    pub fn push_message(&self, message_body: impl Message) {
        let message = AppMessage {
            id_path: self.id_path.clone(),
            body: Box::new(message_body),
        };
        self.app_ref.handle_message(message);
    }
}

/// The [`View`](`crate::core::View`) `Context` which is used for all [`DomView`]s
#[derive(Default)]
pub struct ViewCtx {
    id_path: IdPath,
    app_ref: Option<Box<dyn AppRunner>>,
}

impl ViewCtx {
    pub fn message_thunk(&self) -> MessageThunk {
        MessageThunk {
            id_path: self.id_path.clone(),
            app_ref: self.app_ref.as_ref().unwrap().clone_box(),
        }
    }
    pub(crate) fn set_runner(&mut self, runner: impl AppRunner + 'static) {
        self.app_ref = Some(Box::new(runner));
    }
}

impl ViewPathTracker for ViewCtx {
    fn push_id(&mut self, id: ViewId) {
        self.id_path.push(id);
    }

    fn pop_id(&mut self) {
        self.id_path.pop();
    }

    fn view_path(&mut self) -> &[ViewId] {
        &self.id_path
    }
}
