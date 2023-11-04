use std::marker::PhantomData;

use wasm_bindgen::{JsCast, UnwrapThrowExt};
use xilem_core::{Id, MessageResult, VecSplice};

use crate::{
    sealed::Sealed, vecmap::VecMap, view::DomNode, AttributeValue, ChangeFlags, Cx, Pod, View,
    ViewMarker, ViewSequence,
};

use super::interfaces::{for_all_dom_interface_relatives, Element};

type CowStr = std::borrow::Cow<'static, str>;

/// The state associated with a HTML element `View`.
///
/// Stores handles to the child elements and any child state, as well as attributes and event listeners
pub struct ElementState<ViewSeqState> {
    pub(crate) children_states: ViewSeqState,
    pub(crate) attributes: VecMap<CowStr, AttributeValue>,
    pub(crate) child_elements: Vec<Pod>,
    pub(crate) scratch: Vec<Pod>,
}

// TODO something like the `after_update` of the former `Element` view (likely as a wrapper view instead)

pub struct CustomElement<T, A = (), Children = ()> {
    name: CowStr,
    children: Children,
    #[allow(clippy::type_complexity)]
    phantom: PhantomData<fn() -> (T, A)>,
}

/// Builder function for a custom element view.
pub fn custom_element<T, A, Children: ViewSequence<T, A>>(
    name: impl Into<CowStr>,
    children: Children,
) -> CustomElement<T, A, Children> {
    CustomElement {
        name: name.into(),
        children,
        phantom: PhantomData,
    }
}

impl<T, A, Children> CustomElement<T, A, Children> {
    fn node_name(&self) -> &str {
        &self.name
    }
}

impl<T, A, Children> ViewMarker for CustomElement<T, A, Children> {}
impl<T, A, Children> Sealed for CustomElement<T, A, Children> {}

impl<T, A, Children> View<T, A> for CustomElement<T, A, Children>
where
    Children: ViewSequence<T, A>,
{
    type State = ElementState<Children::State>;

    // This is mostly intended for Autonomous custom elements,
    // TODO: Custom builtin components need some special handling (`document.createElement("p", { is: "custom-component" })`)
    type Element = web_sys::HtmlElement;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let el = cx.create_html_element(&self.name);

        let attributes = cx.apply_attributes(&el);

        let mut child_elements = vec![];
        let (id, children_states) =
            cx.with_new_id(|cx| self.children.build(cx, &mut child_elements));

        for child in &child_elements {
            el.append_child(child.0.as_node_ref()).unwrap_throw();
        }

        // Set the id used internally to the `data-debugid` attribute.
        // This allows the user to see if an element has been re-created or only altered.
        #[cfg(debug_assertions)]
        el.set_attribute("data-debugid", &id.to_raw().to_string())
            .unwrap_throw();

        let el = el.dyn_into().unwrap_throw();
        let state = ElementState {
            children_states,
            child_elements,
            scratch: vec![],
            attributes,
        };
        (id, state, el)
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        id: &mut Id,
        state: &mut Self::State,
        element: &mut Self::Element,
    ) -> ChangeFlags {
        let mut changed = ChangeFlags::empty();

        // update tag name
        if prev.name != self.name {
            // recreate element
            let parent = element
                .parent_element()
                .expect_throw("this element was mounted and so should have a parent");
            parent.remove_child(element).unwrap_throw();
            let new_element = cx.create_html_element(self.node_name());
            // TODO could this be combined with child updates?
            while element.child_element_count() > 0 {
                new_element
                    .append_child(&element.child_nodes().get(0).unwrap_throw())
                    .unwrap_throw();
            }
            *element = new_element.dyn_into().unwrap_throw();
            changed |= ChangeFlags::STRUCTURE;
        }

        changed |= cx.apply_attribute_changes(element, &mut state.attributes);

        // update children
        let mut splice = VecSplice::new(&mut state.child_elements, &mut state.scratch);
        changed |= cx.with_id(*id, |cx| {
            self.children
                .rebuild(cx, &prev.children, &mut state.children_states, &mut splice)
        });
        if changed.contains(ChangeFlags::STRUCTURE) {
            // This is crude and will result in more DOM traffic than needed.
            // The right thing to do is diff the new state of the children id
            // vector against the old, and derive DOM mutations from that.
            while let Some(child) = element.first_child() {
                element.remove_child(&child).unwrap_throw();
            }
            for child in &state.child_elements {
                element.append_child(child.0.as_node_ref()).unwrap_throw();
            }
            changed.remove(ChangeFlags::STRUCTURE);
        }
        changed
    }

    fn message(
        &self,
        id_path: &[Id],
        state: &mut Self::State,
        message: Box<dyn std::any::Any>,
        app_state: &mut T,
    ) -> MessageResult<A> {
        self.children
            .message(id_path, &mut state.children_states, message, app_state)
    }
}

impl<T, A, Children: ViewSequence<T, A>> Element<T, A> for CustomElement<T, A, Children> {}
impl<T, A, Children: ViewSequence<T, A>> crate::interfaces::HtmlElement<T, A>
    for CustomElement<T, A, Children>
{
}

macro_rules! generate_dom_interface_impl {
    ($dom_interface:ident, $ty_name:ident, $t:ident, $a:ident, $vs:ident) => {
        impl<$t, $a, $vs> $crate::interfaces::$dom_interface<$t, $a> for $ty_name<$t, $a, $vs> where
            $vs: $crate::view::ViewSequence<$t, $a>
        {
        }
    };
}

// TODO maybe it's possible to reduce even more in the impl function bodies and put into impl_functions
//      (should improve compile times and probably wasm binary size)
macro_rules! define_html_element {
    (($ty_name:ident, $name:ident, $dom_interface:ident)) => {
        define_html_element!(($ty_name, $name, $dom_interface, T, A, VS));
    };
    (($ty_name:ident, $name:ident, $dom_interface:ident, $t:ident, $a: ident, $vs: ident)) => {
        pub struct $ty_name<$t, $a = (), $vs = ()>($vs, PhantomData<fn() -> ($t, $a)>);

        impl<$t, $a, $vs> ViewMarker for $ty_name<$t, $a, $vs> {}
        impl<$t, $a, $vs> Sealed for $ty_name<$t, $a, $vs> {}

        impl<$t, $a, $vs: ViewSequence<$t, $a>> View<$t, $a> for $ty_name<$t, $a, $vs> {
            type State = ElementState<$vs::State>;
            type Element = web_sys::$dom_interface;

            fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
                let el = cx.create_html_element(stringify!($name));

                let attributes = cx.apply_attributes(&el);

                let mut child_elements = vec![];
                let (id, children_states) =
                    cx.with_new_id(|cx| self.0.build(cx, &mut child_elements));
                for child in &child_elements {
                    el.append_child(child.0.as_node_ref()).unwrap_throw();
                }

                // Set the id used internally to the `data-debugid` attribute.
                // This allows the user to see if an element has been re-created or only altered.
                #[cfg(debug_assertions)]
                el.set_attribute("data-debugid", &id.to_raw().to_string())
                    .unwrap_throw();

                let el = el.dyn_into().unwrap_throw();
                let state = ElementState {
                    children_states,
                    child_elements,
                    scratch: vec![],
                    attributes,
                };
                (id, state, el)
            }

            fn rebuild(
                &self,
                cx: &mut Cx,
                prev: &Self,
                id: &mut Id,
                state: &mut Self::State,
                element: &mut Self::Element,
            ) -> ChangeFlags {
                let mut changed = ChangeFlags::empty();

                changed |= cx.apply_attribute_changes(element, &mut state.attributes);

                // update children
                let mut splice = VecSplice::new(&mut state.child_elements, &mut state.scratch);
                changed |= cx.with_id(*id, |cx| {
                    self.0
                        .rebuild(cx, &prev.0, &mut state.children_states, &mut splice)
                });
                if changed.contains(ChangeFlags::STRUCTURE) {
                    // This is crude and will result in more DOM traffic than needed.
                    // The right thing to do is diff the new state of the children id
                    // vector against the old, and derive DOM mutations from that.
                    while let Some(child) = element.first_child() {
                        element.remove_child(&child).unwrap_throw();
                    }
                    for child in &state.child_elements {
                        element.append_child(child.0.as_node_ref()).unwrap_throw();
                    }
                    changed.remove(ChangeFlags::STRUCTURE);
                }
                changed
            }

            fn message(
                &self,
                id_path: &[Id],
                state: &mut Self::State,
                message: Box<dyn std::any::Any>,
                app_state: &mut $t,
            ) -> MessageResult<$a> {
                self.0
                    .message(id_path, &mut state.children_states, message, app_state)
            }
        }

        /// Builder function for a
        #[doc = concat!("`", stringify!($name), "`")]
        /// element view.
        pub fn $name<$t, $a, $vs: ViewSequence<$t, $a>>(children: $vs) -> $ty_name<$t, $a, $vs> {
            $ty_name(children, PhantomData)
        }

        for_all_dom_interface_relatives!(
            $dom_interface,
            generate_dom_interface_impl,
            $ty_name,
            $t,
            $a,
            $vs
        );
    };
}

macro_rules! define_elements {
    ($($element_def:tt,)*) => {
        $(define_html_element!($element_def);)*
    };
}

define_elements!(
    // the order is copied from
    // https://developer.mozilla.org/en-US/docs/Web/HTML/Element
    // DOM interfaces copied from https://html.spec.whatwg.org/multipage/grouping-content.html and friends

    // content sectioning
    (Address, address, HtmlElement),
    (Article, article, HtmlElement),
    (Aside, aside, HtmlElement),
    (Footer, footer, HtmlElement),
    (Header, header, HtmlElement),
    (H1, h1, HtmlHeadingElement),
    (H2, h2, HtmlHeadingElement),
    (H3, h3, HtmlHeadingElement),
    (H4, h4, HtmlHeadingElement),
    (H5, h5, HtmlHeadingElement),
    (H6, h6, HtmlHeadingElement),
    (Hgroup, hgroup, HtmlElement),
    (Main, main, HtmlElement),
    (Nav, nav, HtmlElement),
    (Section, section, HtmlElement),
    // text content
    (Blockquote, blockquote, HtmlQuoteElement),
    (Dd, dd, HtmlElement),
    (Div, div, HtmlDivElement),
    (Dl, dl, HtmlDListElement),
    (Dt, dt, HtmlElement),
    (Figcaption, figcaption, HtmlElement),
    (Figure, figure, HtmlElement),
    (Hr, hr, HtmlHrElement),
    (Li, li, HtmlLiElement),
    (Menu, menu, HtmlMenuElement),
    (Ol, ol, HtmlOListElement),
    (P, p, HtmlParagraphElement),
    (Pre, pre, HtmlPreElement),
    (Ul, ul, HtmlUListElement),
    // inline text
    (A, a, HtmlAnchorElement, T, A_, VS),
    (Abbr, abbr, HtmlElement),
    (B, b, HtmlElement),
    (Bdi, bdi, HtmlElement),
    (Bdo, bdo, HtmlElement),
    (Br, br, HtmlBrElement),
    (Cite, cite, HtmlElement),
    (Code, code, HtmlElement),
    (Data, data, HtmlDataElement),
    (Dfn, dfn, HtmlElement),
    (Em, em, HtmlElement),
    (I, i, HtmlElement),
    (Kbd, kbd, HtmlElement),
    (Mark, mark, HtmlElement),
    (Q, q, HtmlQuoteElement),
    (Rp, rp, HtmlElement),
    (Rt, rt, HtmlElement),
    (Ruby, ruby, HtmlElement),
    (S, s, HtmlElement),
    (Samp, samp, HtmlElement),
    (Small, small, HtmlElement),
    (Span, span, HtmlSpanElement),
    (Strong, strong, HtmlElement),
    (Sub, sub, HtmlElement),
    (Sup, sup, HtmlElement),
    (Time, time, HtmlTimeElement),
    (U, u, HtmlElement),
    (Var, var, HtmlElement),
    (Wbr, wbr, HtmlElement),
    // image and multimedia
    (Area, area, HtmlAreaElement),
    (Audio, audio, HtmlAudioElement),
    (Canvas, canvas, HtmlCanvasElement),
    (Img, img, HtmlImageElement),
    (Map, map, HtmlMapElement),
    (Track, track, HtmlTrackElement),
    (Video, video, HtmlVideoElement),
    // embedded content
    (Embed, embed, HtmlEmbedElement),
    (Iframe, iframe, HtmlIFrameElement),
    (Object, object, HtmlObjectElement),
    (Picture, picture, HtmlPictureElement),
    (Portal, portal, HtmlElement),
    (Source, source, HtmlSourceElement),
    // SVG and MathML (TODO, svg and mathml elements)
    (Svg, svg, HtmlElement),
    (Math, math, HtmlElement),
    // scripting
    (Noscript, noscript, HtmlElement),
    (Script, script, HtmlScriptElement),
    // demarcating edits
    (Del, del, HtmlModElement),
    (Ins, ins, HtmlModElement),
    // tables
    (Caption, caption, HtmlTableCaptionElement),
    (Col, col, HtmlTableColElement),
    (Colgroup, colgroup, HtmlTableColElement),
    (Table, table, HtmlTableElement),
    (Tbody, tbody, HtmlTableSectionElement),
    (Td, td, HtmlTableCellElement),
    (Tfoot, tfoot, HtmlTableSectionElement),
    (Th, th, HtmlTableCellElement),
    (Thead, thead, HtmlTableSectionElement),
    (Tr, tr, HtmlTableRowElement),
    // forms
    (Button, button, HtmlButtonElement),
    (Datalist, datalist, HtmlDataListElement),
    (Fieldset, fieldset, HtmlFieldSetElement),
    (Form, form, HtmlFormElement),
    (Input, input, HtmlInputElement),
    (Label, label, HtmlLabelElement),
    (Legend, legend, HtmlLegendElement),
    (Meter, meter, HtmlMeterElement),
    (Optgroup, optgroup, HtmlOptGroupElement),
    (OptionElement, option, HtmlOptionElement), // Avoid cluttering the namespace with `Option`
    (Output, output, HtmlOutputElement),
    (Progress, progress, HtmlProgressElement),
    (Select, select, HtmlSelectElement),
    (Textarea, textarea, HtmlTextAreaElement),
    // interactive elements,
    (Details, details, HtmlDetailsElement),
    (Dialog, dialog, HtmlDialogElement),
    (Summary, summary, HtmlElement),
    // web components,
    (Slot, slot, HtmlSlotElement),
    (Template, template, HtmlTemplateElement),
);
