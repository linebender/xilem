use std::marker::PhantomData;

use wasm_bindgen::{JsCast, UnwrapThrowExt};
use xilem_core::{Id, MessageResult, VecSplice};

use crate::{
    interfaces::sealed::Sealed, vecmap::VecMap, view::DomNode, AttributeValue, ChangeFlags, Cx,
    ElementsSplice, Pod, View, ViewMarker, ViewSequence, HTML_NS,
};

use super::interfaces::Element;

type CowStr = std::borrow::Cow<'static, str>;

/// The state associated with a HTML element `View`.
///
/// Stores handles to the child elements and any child state, as well as attributes and event listeners
pub struct ElementState<ViewSeqState> {
    pub(crate) children_states: ViewSeqState,
    pub(crate) attributes: VecMap<CowStr, AttributeValue>,
    pub(crate) child_elements: Vec<Pod>,
    /// This is temporary cache for elements while updating/diffing,
    /// after usage it shouldn't contain any elements,
    /// and is mainly here to avoid unnecessary allocations
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

/// An `ElementsSplice` that does DOM updates in place
struct ChildrenSplice<'a, 'b, 'c> {
    children: VecSplice<'a, 'b, Pod>,
    child_idx: u32,
    parent: &'c web_sys::Node,
    node_list: Option<web_sys::NodeList>,
    prev_element_count: usize,
}

impl<'a, 'b, 'c> ChildrenSplice<'a, 'b, 'c> {
    fn new(
        children: &'a mut Vec<Pod>,
        scratch: &'b mut Vec<Pod>,
        parent: &'c web_sys::Node,
    ) -> Self {
        let prev_element_count = children.len();
        Self {
            children: VecSplice::new(children, scratch),
            child_idx: 0,
            parent,
            node_list: None,
            prev_element_count,
        }
    }
}

impl<'a, 'b, 'c> ElementsSplice for ChildrenSplice<'a, 'b, 'c> {
    fn push(&mut self, element: Pod, _cx: &mut Cx) {
        self.parent
            .append_child(element.0.as_node_ref())
            .unwrap_throw();
        self.child_idx += 1;
        self.children.push(element);
    }

    fn mutate(&mut self, _cx: &mut Cx) -> &mut Pod {
        self.children.mutate()
    }

    fn delete(&mut self, n: usize, _cx: &mut Cx) {
        // Optimization in case all elements are deleted at once
        if n == self.prev_element_count {
            self.parent.set_text_content(None);
        } else {
            // lazy NodeList access, in case it's not necessary at all, which is slightly faster when there's no need for the NodeList
            let node_list = if let Some(node_list) = &self.node_list {
                node_list
            } else {
                self.node_list = Some(self.parent.child_nodes());
                self.node_list.as_ref().unwrap()
            };
            for _ in 0..n {
                let child = node_list.get(self.child_idx).unwrap_throw();
                self.parent.remove_child(&child).unwrap_throw();
            }
        }
        self.children.delete(n);
    }

    fn len(&self, _cx: &mut Cx) -> usize {
        self.children.len()
    }

    fn mark(&mut self, mut changeflags: ChangeFlags, _cx: &mut Cx) -> ChangeFlags {
        if changeflags.contains(ChangeFlags::STRUCTURE) {
            let node_list = if let Some(node_list) = &self.node_list {
                node_list
            } else {
                self.node_list = Some(self.parent.child_nodes());
                self.node_list.as_ref().unwrap()
            };
            let cur_child = self.children.peek_mut().unwrap_throw();
            let old_child = node_list.get(self.child_idx).unwrap_throw();
            self.parent
                .replace_child(cur_child.0.as_node_ref(), &old_child)
                .unwrap_throw();
            // TODO do something else with the structure information?
            changeflags.remove(ChangeFlags::STRUCTURE);
        }
        self.child_idx += 1;
        changeflags
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
        let (el, attributes) = cx.build_element(HTML_NS, &self.name);

        let mut child_elements = vec![];
        let mut scratch = vec![];
        let mut splice = ChildrenSplice::new(&mut child_elements, &mut scratch, &el);

        let (id, children_states) = cx.with_new_id(|cx| self.children.build(cx, &mut splice));

        debug_assert!(scratch.is_empty());

        // Set the id used internally to the `data-debugid` attribute.
        // This allows the user to see if an element has been re-created or only altered.
        #[cfg(debug_assertions)]
        el.set_attribute("data-debugid", &id.to_raw().to_string())
            .unwrap_throw();

        let el = el.dyn_into().unwrap_throw();
        let state = ElementState {
            children_states,
            child_elements,
            scratch,
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
            let (new_element, attributes) = cx.build_element(HTML_NS, self.node_name());
            state.attributes = attributes;
            // TODO could this be combined with child updates?
            while let Some(child) = element.child_nodes().get(0) {
                new_element.append_child(&child).unwrap_throw();
            }
            *element = new_element.dyn_into().unwrap_throw();
            changed |= ChangeFlags::STRUCTURE;
        }

        changed |= cx.rebuild_element(element, &mut state.attributes);

        // update children
        let mut splice =
            ChildrenSplice::new(&mut state.child_elements, &mut state.scratch, element);
        changed |= cx.with_id(*id, |cx| {
            self.children
                .rebuild(cx, &prev.children, &mut state.children_states, &mut splice)
        });
        debug_assert!(state.scratch.is_empty());
        changed.remove(ChangeFlags::STRUCTURE);
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
    ($dom_interface:ident, ($ty_name:ident, $t:ident, $a:ident, $vs:ident)) => {
        impl<$t, $a, $vs> $crate::interfaces::$dom_interface<$t, $a> for $ty_name<$t, $a, $vs> where
            $vs: $crate::view::ViewSequence<$t, $a>
        {
        }
    };
}

// TODO maybe it's possible to reduce even more in the impl function bodies and put into impl_functions
//      (should improve compile times and probably wasm binary size)
macro_rules! define_element {
    ($ns:expr, ($ty_name:ident, $name:ident, $dom_interface:ident)) => {
        define_element!($ns, (
            $ty_name,
            $name,
            $dom_interface,
            stringify!($name),
            T,
            A,
            VS
        ));
    };
    ($ns:expr, ($ty_name:ident, $name:ident, $dom_interface:ident, $tag_name: expr)) => {
        define_element!($ns, (
            $ty_name,
            $name,
            $dom_interface,
            $tag_name,
            T,
            A,
            VS
        ));
    };
    ($ns:expr, ($ty_name:ident, $name:ident, $dom_interface:ident, $tag_name:expr, $t:ident, $a: ident, $vs: ident)) => {
        pub struct $ty_name<$t, $a = (), $vs = ()>($vs, PhantomData<fn() -> ($t, $a)>);

        impl<$t, $a, $vs> ViewMarker for $ty_name<$t, $a, $vs> {}
        impl<$t, $a, $vs> Sealed for $ty_name<$t, $a, $vs> {}

        impl<$t, $a, $vs: ViewSequence<$t, $a>> View<$t, $a> for $ty_name<$t, $a, $vs> {
            type State = ElementState<$vs::State>;
            type Element = web_sys::$dom_interface;

            fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
                let (el, attributes) = cx.build_element($ns, $tag_name);

                let mut child_elements = vec![];
                let mut scratch = vec![];
                let mut splice = ChildrenSplice::new(&mut child_elements, &mut scratch, &el);

                let (id, children_states) = cx.with_new_id(|cx| self.0.build(cx, &mut splice));
                debug_assert!(scratch.is_empty());

                // Set the id used internally to the `data-debugid` attribute.
                // This allows the user to see if an element has been re-created or only altered.
                #[cfg(debug_assertions)]
                el.set_attribute("data-debugid", &id.to_raw().to_string())
                    .unwrap_throw();

                let el = el.dyn_into().unwrap_throw();
                let state = ElementState {
                    children_states,
                    child_elements,
                    scratch,
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

                changed |= cx.rebuild_element(element, &mut state.attributes);

                // update children
                let mut splice = ChildrenSplice::new(&mut state.child_elements, &mut state.scratch, element);
                changed |= cx.with_id(*id, |cx| {
                    self.0.rebuild(cx, &prev.0, &mut state.children_states, &mut splice)
                });
                debug_assert!(state.scratch.is_empty());
                changed.remove(ChangeFlags::STRUCTURE); // this is handled by the ChildrenSplice already
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
        #[doc = concat!("`", $tag_name, "`")]
        /// element view.
        pub fn $name<$t, $a, $vs: ViewSequence<$t, $a>>(children: $vs) -> $ty_name<$t, $a, $vs> {
            $ty_name(children, PhantomData)
        }

        generate_dom_interface_impl!($dom_interface, ($ty_name, $t, $a, $vs));

        paste::paste! {
            $crate::interfaces::[<for_all_ $dom_interface:snake _ancestors>]!(generate_dom_interface_impl, ($ty_name, $t, $a, $vs));
        }
    };
}

macro_rules! define_elements {
    ($ns:ident, $($element_def:tt,)*) => {
        use std::marker::PhantomData;
        use wasm_bindgen::{JsCast, UnwrapThrowExt};
        use xilem_core::{Id, MessageResult};
        use super::{ElementState, ChildrenSplice};

        use crate::{
            interfaces::sealed::Sealed,
            ChangeFlags, Cx, View, ViewMarker, ViewSequence,
        };

        $(define_element!(crate::$ns, $element_def);)*
    };
}

pub mod html {
    define_elements!(
        // the order is copied from
        // https://developer.mozilla.org/en-US/docs/Web/HTML/Element
        // DOM interfaces copied from https://html.spec.whatwg.org/multipage/grouping-content.html and friends

        // TODO include document metadata elements?
        HTML_NS,
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
        (Link, link, HtmlLinkElement),
        (Menu, menu, HtmlMenuElement),
        (Ol, ol, HtmlOListElement),
        (P, p, HtmlParagraphElement),
        (Pre, pre, HtmlPreElement),
        (Ul, ul, HtmlUListElement),
        // inline text
        (A, a, HtmlAnchorElement, "a", T, A_, VS),
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
}

pub mod mathml {
    define_elements!(
        MATHML_NS,
        (Math, math, Element),
        (Annotation, annotation, Element),
        (AnnotationXml, annotation_xml, Element, "annotation-xml"),
        (Maction, maction, Element),
        (Merror, merror, Element),
        (Mfrac, mfrac, Element),
        (Mi, mi, Element),
        (Mmultiscripts, mmultiscripts, Element),
        (Mn, mn, Element),
        (Mo, mo, Element),
        (Mover, mover, Element),
        (Mpadded, mpadded, Element),
        (Mphantom, mphantom, Element),
        (Mprescripts, mprescripts, Element),
        (Mroot, mroot, Element),
        (Mrow, mrow, Element),
        (Ms, ms, Element),
        (Mspace, mspace, Element),
        (Msqrt, msqrt, Element),
        (Mstyle, mstyle, Element),
        (Msub, msub, Element),
        (Msubsup, msubsup, Element),
        (Msup, msup, Element),
        (Mtable, mtable, Element),
        (Mtd, mtd, Element),
        (Mtext, mtext, Element),
        (Mtr, mtr, Element),
        (Munder, munder, Element),
        (Munderover, munderover, Element),
        (Semantics, semantics, Element),
    );
}

pub mod svg {
    define_elements!(
        SVG_NS,
        (Svg, svg, SvgsvgElement),
        (A, a, SvgaElement, "a", T, A_, VS),
        (Animate, animate, SvgAnimateElement),
        (
            AnimateMotion,
            animate_motion,
            SvgAnimateMotionElement,
            "animateMotion"
        ),
        (
            AnimateTransform,
            animate_transform,
            SvgAnimateTransformElement,
            "animateTransform"
        ),
        (Circle, circle, SvgCircleElement),
        (ClipPath, clip_path, SvgClipPathElement, "clipPath"),
        (Defs, defs, SvgDefsElement),
        (Desc, desc, SvgDescElement),
        (Ellipse, ellipse, SvgEllipseElement),
        (FeBlend, fe_blend, SvgfeBlendElement, "feBlend"),
        (
            FeColorMatrix,
            fe_color_matrix,
            SvgfeColorMatrixElement,
            "feColorMatrix"
        ),
        (
            FeComponentTransfer,
            fe_component_transfer,
            SvgfeComponentTransferElement,
            "feComponentTransfer"
        ),
        (
            FeComposite,
            fe_composite,
            SvgfeCompositeElement,
            "feComposite"
        ),
        (
            FeConvolveMatrix,
            fe_convolve_matrix,
            SvgfeConvolveMatrixElement,
            "feConvolveMatrix"
        ),
        (
            FeDiffuseLighting,
            fe_diffuse_lighting,
            SvgfeDiffuseLightingElement,
            "feDiffuseLighting"
        ),
        (
            FeDisplacementMap,
            fe_displacement_map,
            SvgfeDisplacementMapElement,
            "feDisplacementMap"
        ),
        (
            FeDistantLight,
            fe_distant_light,
            SvgfeDistantLightElement,
            "feDistantLight"
        ),
        (
            FeDropShadow,
            fe_drop_shadow,
            SvgfeDropShadowElement,
            "feDropShadow"
        ),
        (FeFlood, fe_flood, SvgfeFloodElement, "feFlood"),
        (FeFuncA, fe_func_a, SvgfeFuncAElement, "feFuncA"),
        (FeFuncB, fe_func_b, SvgfeFuncBElement, "feFuncB"),
        (FeFuncG, fe_func_g, SvgfeFuncGElement, "feFuncG"),
        (FeFuncR, fe_func_r, SvgfeFuncRElement, "feFuncR"),
        (
            FeGaussianBlur,
            fe_gaussian_blur,
            SvgfeGaussianBlurElement,
            "feGaussianBlur"
        ),
        (FeImage, fe_image, SvgfeImageElement, "feImage"),
        (FeMerge, fe_merge, SvgfeMergeElement, "feMerge"),
        (
            FeMergeNode,
            fe_merge_node,
            SvgfeMergeNodeElement,
            "feMergeNode"
        ),
        (
            FeMorphology,
            fe_morphology,
            SvgfeMorphologyElement,
            "feMorphology"
        ),
        (FeOffset, fe_offset, SvgfeOffsetElement, "feOffset"),
        (
            FePointLight,
            fe_point_light,
            SvgfePointLightElement,
            "fePointLight"
        ),
        (
            FeSpecularLighting,
            fe_specular_lighting,
            SvgfeSpecularLightingElement,
            "feSpecularLighting"
        ),
        (
            FeSpotLight,
            fe_spot_light,
            SvgfeSpotLightElement,
            "feSpotLight"
        ),
        (FeTile, fe_tile, SvgfeTileElement, "feTile"),
        (
            FeTurbulence,
            fe_turbulence,
            SvgfeTurbulenceElement,
            "feTurbulence"
        ),
        (Filter, filter, SvgFilterElement),
        (
            ForeignObject,
            foreign_object,
            SvgForeignObjectElement,
            "foreignObject"
        ),
        (G, g, SvggElement),
        // (Hatch, hatch, SvgHatchElement),
        // (Hatchpath, hatchpath, SvgHatchpathElement),
        (Image, image, SvgImageElement),
        (Line, line, SvgLineElement),
        (
            LinearGradient,
            linear_gradient,
            SvgLinearGradientElement,
            "linearGradient"
        ),
        (Marker, marker, SvgMarkerElement),
        (Mask, mask, SvgMaskElement),
        (Metadata, metadata, SvgMetadataElement),
        (Mpath, mpath, SvgmPathElement),
        (Path, path, SvgPathElement),
        (Pattern, pattern, SvgPatternElement),
        (Polygon, polygon, SvgPolygonElement),
        (Polyline, polyline, SvgPolylineElement),
        (
            RadialGradient,
            radial_gradient,
            SvgRadialGradientElement,
            "radialGradient"
        ),
        (Rect, rect, SvgRectElement),
        (ScriptSvg, script_svg, SvgScriptElement),
        (Set, set, SvgSetElement),
        (Stop, stop, SvgStopElement),
        (Style, style, SvgStyleElement),
        (Switch, switch, SvgSwitchElement),
        (Symbol, symbol, SvgSymbolElement),
        (Text, text, SvgTextElement),
        (TextPath, text_path, SvgTextPathElement, "textPath"),
        (Title, title, SvgTitleElement),
        (Tspan, tspan, SvgtSpanElement),
        (Use, use_, SvgUseElement),
        (SvgView, view, SvgViewElement),
    );
}
