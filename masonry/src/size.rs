#[derive(Clone, Copy, Default, PartialEq)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Size<T>  {
    /// The width.
    pub width: T,
    /// The height.
    pub height: T,
}