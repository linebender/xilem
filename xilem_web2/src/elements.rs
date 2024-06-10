use wasm_bindgen::UnwrapThrowExt;
// use web_sys::console;
use xilem_core::{AppendVec, ElementSplice, Mut};

use crate::{vec_splice::VecSplice, DynNode, Pod};

pub struct ElementState<SeqState> {
    seq_state: SeqState,
    append_scratch: AppendVec<Pod<DynNode>>,
    vec_splice_scratch: Vec<Pod<DynNode>>,
}

impl<SeqState> ElementState<SeqState> {
    pub fn new(seq_state: SeqState) -> Self {
        Self {
            seq_state,
            append_scratch: Default::default(),
            vec_splice_scratch: Default::default(),
        }
    }
}

// An alternative idea for this would be to track all the changes (via a `Vec<ChildMutation>`)
// and apply them at once, when this splice is being `Drop`ped, needs some investigation, whether that's better than in place mutations
// TODO maybe we can save some allocations/memory (this needs two extra `Vec`s)
struct DomChildrenSplice<'a, 'b, 'c, 'd> {
    scratch: &'a mut AppendVec<Pod<DynNode>>,
    children: VecSplice<'b, 'c, Pod<DynNode>>,
    ix: usize,
    parent: &'d web_sys::Node,
    parent_was_removed: bool,
}

impl<'a, 'b, 'c, 'd> DomChildrenSplice<'a, 'b, 'c, 'd> {
    fn new(
        scratch: &'a mut AppendVec<Pod<DynNode>>,
        children: &'b mut Vec<Pod<DynNode>>,
        vec_splice_scratch: &'c mut Vec<Pod<DynNode>>,
        parent: &'d web_sys::Node,
        parent_was_deleted: bool,
    ) -> Self {
        Self {
            scratch,
            children: VecSplice::new(children, vec_splice_scratch),
            ix: 0,
            parent,
            parent_was_removed: parent_was_deleted,
        }
    }
}

impl<'a, 'b, 'c, 'd> ElementSplice<Pod<DynNode>> for DomChildrenSplice<'a, 'b, 'c, 'd> {
    fn with_scratch<R>(&mut self, f: impl FnOnce(&mut AppendVec<Pod<DynNode>>) -> R) -> R {
        let ret = f(self.scratch);
        for element in self.scratch.drain() {
            // can't use self.push because borrow-checker...
            self.parent
                .append_child(element.node.as_ref())
                .unwrap_throw();
            self.children.insert(element);
            self.ix += 1;
        }
        ret
    }

    fn insert(&mut self, element: Pod<DynNode>) {
        self.parent
            .insert_before(
                element.node.as_ref(),
                self.children.next_mut().map(|p| p.node.as_ref()),
            )
            .unwrap_throw();
        self.ix += 1;
        self.children.insert(element);
    }

    fn mutate<R>(&mut self, f: impl FnOnce(Mut<'_, Pod<DynNode>>) -> R) -> R {
        let child = self.children.mutate();
        let ret = f(child.as_mut(self.parent_was_removed));
        self.ix += 1;
        ret
    }

    fn skip(&mut self, n: usize) {
        self.children.skip(n);
        self.ix += n;
    }

    fn delete<R>(&mut self, f: impl FnOnce(Mut<'_, Pod<DynNode>>) -> R) -> R {
        let mut child = self.children.delete_next();
        let child = child.as_mut(true);
        // child.was_removed = true;
        // TODO: Should the child cleanup and remove itself from its parent?
        // TODO: Should the parent be kept for the child before invoking `f`?
        // This is an optimization to avoid too much DOM traffic, otherwise first the children would be deleted from that node in an up-traversal
        if !self.parent_was_removed {
            self.parent
                .remove_child(child.node.as_ref())
                .ok()
                .unwrap_throw();
        }
        f(child)
    }
}

macro_rules! define_element {
    ($ns:expr, ($ty_name:ident, $name:ident, $dom_interface:ident)) => {
        define_element!(
            $ns,
            (
                $ty_name,
                $name,
                $dom_interface,
                stringify!($name),
                State,
                Action,
                Children,
                Marker
            )
        );
    };
    ($ns:expr, ($ty_name:ident, $name:ident, $dom_interface:ident, $tag_name: expr)) => {
        define_element!(
            $ns,
            (
                $ty_name,
                $name,
                $dom_interface,
                $tag_name,
                State,
                Action,
                Children,
                Marker
            )
        );
    };
    ($ns:expr, ($ty_name:ident, $name:ident, $dom_interface:ident, $tag_name:expr, $t:ident, $a: ident, $vs: ident, $marker: ident)) => {
        pub struct $ty_name<$vs, $marker> {
            children: $vs,
            phantom: PhantomData<fn() -> $marker>,
        }

        /// Builder function for a
        #[doc = concat!("`", $tag_name, "`")]
        /// element view.
        pub fn $name<$vs, $marker>(children: $vs) -> $ty_name<$vs, $marker> {
            $ty_name {
                children,
                phantom: PhantomData,
            }
        }

        impl<
                $t,
                $a,
                $marker: 'static,
                $vs: ViewSequence<$t, $a, ViewCtx, Pod<DynNode>, $marker>,
            > View<$t, $a, ViewCtx> for $ty_name<$vs, $marker>
        {
            type Element = Pod<web_sys::$dom_interface>; // TODO correct type

            type ViewState = ElementState<$vs::SeqState>;

            fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
                let mut elements = AppendVec::default();
                let state = ElementState::new(self.children.seq_build(ctx, &mut elements));
                (
                    Pod::new_element(elements.into_inner(), $ns, $tag_name).into(),
                    state,
                )
            }

            fn rebuild<'el>(
                &self,
                prev: &Self,
                element_state: &mut Self::ViewState,
                ctx: &mut ViewCtx,
                element: Mut<'el, Self::Element>,
            ) -> Mut<'el, Self::Element> {
                let children_empty = self.children.count(&element_state.seq_state) == 0;
                // TODO this is maybe not optimal in the sense that teardown is not called on the children views, but it's more performant,
                // TODO Thoroughly test whether the teardown for children is really needed...
                if children_empty && prev.children.count(&element_state.seq_state) != 0 {
                    element.node.set_text_content(None);
                    element.props.children.clear();
                    return element;
                }
                let mut dom_children_splice = DomChildrenSplice::new(
                    &mut element_state.append_scratch,
                    &mut element.props.children,
                    &mut element_state.vec_splice_scratch,
                    element.node,
                    element.was_removed || children_empty,
                );
                self.children.seq_rebuild(
                    &prev.children,
                    &mut element_state.seq_state,
                    ctx,
                    &mut dom_children_splice,
                );
                element
            }

            fn teardown(
                &self,
                element_state: &mut Self::ViewState,
                ctx: &mut ViewCtx,
                element: Mut<'_, Self::Element>,
            ) {
                let mut dom_children_splice = DomChildrenSplice::new(
                    &mut element_state.append_scratch,
                    &mut element.props.children,
                    &mut element_state.vec_splice_scratch,
                    element.node,
                    element.was_removed,
                );
                self.children.seq_teardown(
                    &mut element_state.seq_state,
                    ctx,
                    &mut dom_children_splice,
                );
            }

            fn message(
                &self,
                view_state: &mut Self::ViewState,
                id_path: &[ViewId],
                message: DynMessage,
                app_state: &mut $t,
            ) -> MessageResult<$a> {
                self.children
                    .seq_message(&mut view_state.seq_state, id_path, message, app_state)
            }
        }
    };
}

macro_rules! define_elements {
    ($ns:ident, $($element_def:tt,)*) => {
        use std::marker::PhantomData;
        // use wasm_bindgen::{JsCast, UnwrapThrowExt};
        use xilem_core::{AppendVec, Mut, DynMessage, ViewId, MessageResult};
        use super::{DomChildrenSplice, ElementState};

        use crate::{ Pod, DynNode, ViewCtx, View, ViewSequence };

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
        (A, a, HtmlAnchorElement, "a", T, A_, VS, Marker),
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
        (A, a, SvgaElement, "a", T, A_, VS, Marker),
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
        (
            Marker,
            marker,
            SvgMarkerElement,
            "marker",
            T,
            A_,
            VS,
            Marker_
        ),
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
