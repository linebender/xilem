mod generated;
pub use generated::*;

use crate::{vecmap::VecMap, AttributeValue, Pod};

type CowStr = std::borrow::Cow<'static, str>;

// TODO: could be split to struct without generic parameter (to avoid monomorphized bloat (methods below))
/// The state associated with a HTML element `View`.
///
/// Stores handles to the child elements and any child state, as well as attributes and event listeners
pub struct ElementState<ViewSeqState> {
    pub(crate) children_states: ViewSeqState,
    pub(crate) attributes: VecMap<CowStr, AttributeValue>,
    pub(crate) child_elements: Vec<Pod>,
    pub(crate) scratch: Vec<Pod>,
}
