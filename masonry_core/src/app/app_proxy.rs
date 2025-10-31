use crate::app::RenderRootSignal;

/// Provides access to signal queue of the app's root state.
pub trait AppProxy {
    /// Send a [`RenderRootSignal`] to the runner of this app, which allows global actions to be triggered by a widget.
    fn emit_render_root_signal(&self, signal: RenderRootSignal);

    /// Send a generic signal to the runner of this app, which allows global actions to be triggered by a widget.
    fn emit_app_signal(&self, signal: Box<dyn std::any::Any>);
}
