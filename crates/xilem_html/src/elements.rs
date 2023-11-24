use std::marker::PhantomData;

use wasm_bindgen::{JsCast, UnwrapThrowExt};
use xilem_core::{Id, MessageResult, VecSplice};

use crate::{
    interfaces::sealed::Sealed, vecmap::VecMap, view::DomNode, AttributeValue, ChangeFlags, Cx,
    Pod, View, ViewMarker, ViewSequence, HTML_NS, MATHML_NS, SVG_NS,
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
        let (el, attributes) = cx.build_element(HTML_NS, &self.name);

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
            let (new_element, attributes) = cx.build_element(HTML_NS, self.node_name());
            state.attributes = attributes;
            // TODO could this be combined with child updates?
            while element.child_element_count() > 0 {
                new_element
                    .append_child(&element.child_nodes().get(0).unwrap_throw())
                    .unwrap_throw();
            }
            *element = new_element.dyn_into().unwrap_throw();
            changed |= ChangeFlags::STRUCTURE;
        }

        changed |= cx.rebuild_element(element, &mut state.attributes);

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
    (($ns:expr, $ty_name:ident, $name:ident, $dom_interface:ident)) => {
        define_element!((
            $ns,
            $ty_name,
            $name,
            $dom_interface,
            stringify!($name),
            T,
            A,
            VS
        ));
    };
    (($ns:expr, $ty_name:ident, $name:ident, $dom_interface:ident, $tag_name: expr)) => {
        define_element!((
            $ns,
            $ty_name,
            $name,
            $dom_interface,
            $tag_name,
            T,
            A,
            VS
        ));
    };
    (($ns:expr, $ty_name:ident, $name:ident, $dom_interface:ident, $tag_name:expr, $t:ident, $a: ident, $vs: ident)) => {
        pub struct $ty_name<$t, $a = (), $vs = ()>($vs, PhantomData<fn() -> ($t, $a)>);

        impl<$t, $a, $vs> ViewMarker for $ty_name<$t, $a, $vs> {}
        impl<$t, $a, $vs> Sealed for $ty_name<$t, $a, $vs> {}

        impl<$t, $a, $vs: ViewSequence<$t, $a>> View<$t, $a> for $ty_name<$t, $a, $vs> {
            type State = ElementState<$vs::State>;
            type Element = web_sys::$dom_interface;

            fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
                let (el, attributes) = cx.build_element($ns, $tag_name);

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
    ($($element_def:tt,)*) => {
        $(define_element!($element_def);)*
    };
}

define_elements!(
    // the order is copied from
    // https://developer.mozilla.org/en-US/docs/Web/HTML/Element
    // DOM interfaces copied from https://html.spec.whatwg.org/multipage/grouping-content.html and friends

    // TODO include document metadata elements?

    // content sectioning
    (HTML_NS, Address, address, HtmlElement),
    (HTML_NS, Article, article, HtmlElement),
    (HTML_NS, Aside, aside, HtmlElement),
    (HTML_NS, Footer, footer, HtmlElement),
    (HTML_NS, Header, header, HtmlElement),
    (HTML_NS, H1, h1, HtmlHeadingElement),
    (HTML_NS, H2, h2, HtmlHeadingElement),
    (HTML_NS, H3, h3, HtmlHeadingElement),
    (HTML_NS, H4, h4, HtmlHeadingElement),
    (HTML_NS, H5, h5, HtmlHeadingElement),
    (HTML_NS, H6, h6, HtmlHeadingElement),
    (HTML_NS, Hgroup, hgroup, HtmlElement),
    (HTML_NS, Main, main, HtmlElement),
    (HTML_NS, Nav, nav, HtmlElement),
    (HTML_NS, Section, section, HtmlElement),
    // text content
    (HTML_NS, Blockquote, blockquote, HtmlQuoteElement),
    (HTML_NS, Dd, dd, HtmlElement),
    (HTML_NS, Div, div, HtmlDivElement),
    (HTML_NS, Dl, dl, HtmlDListElement),
    (HTML_NS, Dt, dt, HtmlElement),
    (HTML_NS, Figcaption, figcaption, HtmlElement),
    (HTML_NS, Figure, figure, HtmlElement),
    (HTML_NS, Hr, hr, HtmlHrElement),
    (HTML_NS, Li, li, HtmlLiElement),
    (HTML_NS, Link, link, HtmlLinkElement),
    (HTML_NS, Menu, menu, HtmlMenuElement),
    (HTML_NS, Ol, ol, HtmlOListElement),
    (HTML_NS, P, p, HtmlParagraphElement),
    (HTML_NS, Pre, pre, HtmlPreElement),
    (HTML_NS, Ul, ul, HtmlUListElement),
    // inline text
    (HTML_NS, A, a, HtmlAnchorElement, "a", T, A_, VS),
    (HTML_NS, Abbr, abbr, HtmlElement),
    (HTML_NS, B, b, HtmlElement),
    (HTML_NS, Bdi, bdi, HtmlElement),
    (HTML_NS, Bdo, bdo, HtmlElement),
    (HTML_NS, Br, br, HtmlBrElement),
    (HTML_NS, Cite, cite, HtmlElement),
    (HTML_NS, Code, code, HtmlElement),
    (HTML_NS, Data, data, HtmlDataElement),
    (HTML_NS, Dfn, dfn, HtmlElement),
    (HTML_NS, Em, em, HtmlElement),
    (HTML_NS, I, i, HtmlElement),
    (HTML_NS, Kbd, kbd, HtmlElement),
    (HTML_NS, Mark, mark, HtmlElement),
    (HTML_NS, Q, q, HtmlQuoteElement),
    (HTML_NS, Rp, rp, HtmlElement),
    (HTML_NS, Rt, rt, HtmlElement),
    (HTML_NS, Ruby, ruby, HtmlElement),
    (HTML_NS, S, s, HtmlElement),
    (HTML_NS, Samp, samp, HtmlElement),
    (HTML_NS, Small, small, HtmlElement),
    (HTML_NS, Span, span, HtmlSpanElement),
    (HTML_NS, Strong, strong, HtmlElement),
    (HTML_NS, Sub, sub, HtmlElement),
    (HTML_NS, Sup, sup, HtmlElement),
    (HTML_NS, Time, time, HtmlTimeElement),
    (HTML_NS, U, u, HtmlElement),
    (HTML_NS, Var, var, HtmlElement),
    (HTML_NS, Wbr, wbr, HtmlElement),
    // image and multimedia
    (HTML_NS, Area, area, HtmlAreaElement),
    (HTML_NS, Audio, audio, HtmlAudioElement),
    (HTML_NS, Canvas, canvas, HtmlCanvasElement),
    (HTML_NS, Img, img, HtmlImageElement),
    (HTML_NS, Map, map, HtmlMapElement),
    (HTML_NS, Track, track, HtmlTrackElement),
    (HTML_NS, Video, video, HtmlVideoElement),
    // embedded content
    (HTML_NS, Embed, embed, HtmlEmbedElement),
    (HTML_NS, Iframe, iframe, HtmlIFrameElement),
    (HTML_NS, Object, object, HtmlObjectElement),
    (HTML_NS, Picture, picture, HtmlPictureElement),
    (HTML_NS, Portal, portal, HtmlElement),
    (HTML_NS, Source, source, HtmlSourceElement),
    // scripting
    (HTML_NS, Noscript, noscript, HtmlElement),
    (HTML_NS, Script, script, HtmlScriptElement),
    // demarcating edits
    (HTML_NS, Del, del, HtmlModElement),
    (HTML_NS, Ins, ins, HtmlModElement),
    // tables
    (HTML_NS, Caption, caption, HtmlTableCaptionElement),
    (HTML_NS, Col, col, HtmlTableColElement),
    (HTML_NS, Colgroup, colgroup, HtmlTableColElement),
    (HTML_NS, Table, table, HtmlTableElement),
    (HTML_NS, Tbody, tbody, HtmlTableSectionElement),
    (HTML_NS, Td, td, HtmlTableCellElement),
    (HTML_NS, Tfoot, tfoot, HtmlTableSectionElement),
    (HTML_NS, Th, th, HtmlTableCellElement),
    (HTML_NS, Thead, thead, HtmlTableSectionElement),
    (HTML_NS, Tr, tr, HtmlTableRowElement),
    // forms
    (HTML_NS, Button, button, HtmlButtonElement),
    (HTML_NS, Datalist, datalist, HtmlDataListElement),
    (HTML_NS, Fieldset, fieldset, HtmlFieldSetElement),
    (HTML_NS, Form, form, HtmlFormElement),
    (HTML_NS, Input, input, HtmlInputElement),
    (HTML_NS, Label, label, HtmlLabelElement),
    (HTML_NS, Legend, legend, HtmlLegendElement),
    (HTML_NS, Meter, meter, HtmlMeterElement),
    (HTML_NS, Optgroup, optgroup, HtmlOptGroupElement),
    (HTML_NS, OptionElement, option, HtmlOptionElement), // Avoid cluttering the namespace with `Option`
    (HTML_NS, Output, output, HtmlOutputElement),
    (HTML_NS, Progress, progress, HtmlProgressElement),
    (HTML_NS, Select, select, HtmlSelectElement),
    (HTML_NS, Textarea, textarea, HtmlTextAreaElement),
    // interactive elements,
    (HTML_NS, Details, details, HtmlDetailsElement),
    (HTML_NS, Dialog, dialog, HtmlDialogElement),
    (HTML_NS, Summary, summary, HtmlElement),
    // web components,
    (HTML_NS, Slot, slot, HtmlSlotElement),
    (HTML_NS, Template, template, HtmlTemplateElement),
    (MATHML_NS, Math, math, Element),
    (MATHML_NS, Annotation, annotation, Element),
    (
        MATHML_NS,
        AnnotationXml,
        annotation_xml,
        Element,
        "annotation-xml"
    ),
    (MATHML_NS, Maction, maction, Element),
    (MATHML_NS, Merror, merror, Element),
    (MATHML_NS, Mfrac, mfrac, Element),
    (MATHML_NS, Mi, mi, Element),
    (MATHML_NS, Mmultiscripts, mmultiscripts, Element),
    (MATHML_NS, Mn, mn, Element),
    (MATHML_NS, Mo, mo, Element),
    (MATHML_NS, Mover, mover, Element),
    (MATHML_NS, Mpadded, mpadded, Element),
    (MATHML_NS, Mphantom, mphantom, Element),
    (MATHML_NS, Mprescripts, mprescripts, Element),
    (MATHML_NS, Mroot, mroot, Element),
    (MATHML_NS, Mrow, mrow, Element),
    (MATHML_NS, Ms, ms, Element),
    (MATHML_NS, Mspace, mspace, Element),
    (MATHML_NS, Msqrt, msqrt, Element),
    (MATHML_NS, Mstyle, mstyle, Element),
    (MATHML_NS, Msub, msub, Element),
    (MATHML_NS, Msubsup, msubsup, Element),
    (MATHML_NS, Msup, msup, Element),
    (MATHML_NS, Mtable, mtable, Element),
    (MATHML_NS, Mtd, mtd, Element),
    (MATHML_NS, Mtext, mtext, Element),
    (MATHML_NS, Mtr, mtr, Element),
    (MATHML_NS, Munder, munder, Element),
    (MATHML_NS, Munderover, munderover, Element),
    (MATHML_NS, Semantics, semantics, Element),
    (SVG_NS, Svg, svg, SvgsvgElement),
    (SVG_NS, Asvg, a_svg, SvgaElement), // TODO is there a better way/name for this <a> element?
    (SVG_NS, Animate, animate, SvgAnimateElement),
    (
        SVG_NS,
        AnimateMotion,
        animate_motion,
        SvgAnimateMotionElement,
        "animateMotion"
    ),
    (
        SVG_NS,
        AnimateTransform,
        animate_transform,
        SvgAnimateTransformElement,
        "animateTransform"
    ),
    (SVG_NS, Circle, circle, SvgCircleElement),
    (SVG_NS, ClipPath, clip_path, SvgClipPathElement, "clipPath"),
    (SVG_NS, Defs, defs, SvgDefsElement),
    (SVG_NS, Desc, desc, SvgDescElement),
    (SVG_NS, Ellipse, ellipse, SvgEllipseElement),
    (SVG_NS, FeBlend, fe_blend, SvgfeBlendElement, "feBlend"),
    (
        SVG_NS,
        FeColorMatrix,
        fe_color_matrix,
        SvgfeColorMatrixElement,
        "feColorMatrix"
    ),
    (
        SVG_NS,
        FeComponentTransfer,
        fe_component_transfer,
        SvgfeComponentTransferElement,
        "feComponentTransfer"
    ),
    (
        SVG_NS,
        FeComposite,
        fe_composite,
        SvgfeCompositeElement,
        "feComposite"
    ),
    (
        SVG_NS,
        FeConvolveMatrix,
        fe_convolve_matrix,
        SvgfeConvolveMatrixElement,
        "feConvolveMatrix"
    ),
    (
        SVG_NS,
        FeDiffuseLighting,
        fe_diffuse_lighting,
        SvgfeDiffuseLightingElement,
        "feDiffuseLighting"
    ),
    (
        SVG_NS,
        FeDisplacementMap,
        fe_displacement_map,
        SvgfeDisplacementMapElement,
        "feDisplacementMap"
    ),
    (
        SVG_NS,
        FeDistantLight,
        fe_distant_light,
        SvgfeDistantLightElement,
        "feDistantLight"
    ),
    (
        SVG_NS,
        FeDropShadow,
        fe_drop_shadow,
        SvgfeDropShadowElement,
        "feDropShadow"
    ),
    (SVG_NS, FeFlood, fe_flood, SvgfeFloodElement, "feFlood"),
    (SVG_NS, FeFuncA, fe_func_a, SvgfeFuncAElement, "feFuncA"),
    (SVG_NS, FeFuncB, fe_func_b, SvgfeFuncBElement, "feFuncB"),
    (SVG_NS, FeFuncG, fe_func_g, SvgfeFuncGElement, "feFuncG"),
    (SVG_NS, FeFuncR, fe_func_r, SvgfeFuncRElement, "feFuncR"),
    (
        SVG_NS,
        FeGaussianBlur,
        fe_gaussian_blur,
        SvgfeGaussianBlurElement,
        "feGaussianBlur"
    ),
    (SVG_NS, FeImage, fe_image, SvgfeImageElement, "feImage"),
    (SVG_NS, FeMerge, fe_merge, SvgfeMergeElement, "feMerge"),
    (
        SVG_NS,
        FeMergeNode,
        fe_merge_node,
        SvgfeMergeNodeElement,
        "feMergeNode"
    ),
    (
        SVG_NS,
        FeMorphology,
        fe_morphology,
        SvgfeMorphologyElement,
        "feMorphology"
    ),
    (SVG_NS, FeOffset, fe_offset, SvgfeOffsetElement, "feOffset"),
    (
        SVG_NS,
        FePointLight,
        fe_point_light,
        SvgfePointLightElement,
        "fePointLight"
    ),
    (
        SVG_NS,
        FeSpecularLighting,
        fe_specular_lighting,
        SvgfeSpecularLightingElement,
        "feSpecularLighting"
    ),
    (
        SVG_NS,
        FeSpotLight,
        fe_spot_light,
        SvgfeSpotLightElement,
        "feSpotLight"
    ),
    (SVG_NS, FeTile, fe_tile, SvgfeTileElement, "feTile"),
    (
        SVG_NS,
        FeTurbulence,
        fe_turbulence,
        SvgfeTurbulenceElement,
        "feTurbulence"
    ),
    (SVG_NS, Filter, filter, SvgFilterElement),
    (
        SVG_NS,
        ForeignObject,
        foreign_object,
        SvgForeignObjectElement,
        "foreignObject"
    ),
    (SVG_NS, G, g, SvggElement),
    // (SVG_NS, Hatch, hatch, SvgHatchElement),
    // (SVG_NS, Hatchpath, hatchpath, SvgHatchpathElement),
    (SVG_NS, Image, image, SvgImageElement),
    (SVG_NS, Line, line, SvgLineElement),
    (
        SVG_NS,
        LinearGradient,
        linear_gradient,
        SvgLinearGradientElement,
        "linearGradient"
    ),
    (SVG_NS, Marker, marker, SvgMarkerElement),
    (SVG_NS, Mask, mask, SvgMaskElement),
    (SVG_NS, Metadata, metadata, SvgMetadataElement),
    (SVG_NS, Mpath, mpath, SvgmPathElement),
    (SVG_NS, Path, path, SvgPathElement),
    (SVG_NS, Pattern, pattern, SvgPatternElement),
    (SVG_NS, Polygon, polygon, SvgPolygonElement),
    (SVG_NS, Polyline, polyline, SvgPolylineElement),
    (
        SVG_NS,
        RadialGradient,
        radial_gradient,
        SvgRadialGradientElement,
        "radialGradient"
    ),
    (SVG_NS, Rect, rect, SvgRectElement),
    (SVG_NS, ScriptSvg, script_svg, SvgScriptElement),
    (SVG_NS, Set, set, SvgSetElement),
    (SVG_NS, Stop, stop, SvgStopElement),
    (SVG_NS, Style, style, SvgStyleElement),
    (SVG_NS, Switch, switch, SvgSwitchElement),
    (SVG_NS, Symbol, symbol, SvgSymbolElement),
    (SVG_NS, Text, text, SvgTextElement),
    (SVG_NS, TextPath, text_path, SvgTextPathElement, "textPath"),
    (SVG_NS, Title, title, SvgTitleElement),
    (SVG_NS, Tspan, tspan, SvgtSpanElement),
    (SVG_NS, Use, use_, SvgUseElement),
    (SVG_NS, SvgView, view, SvgViewElement),
);
