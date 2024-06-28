// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Basic builder functions to create DOM elements, such as [`html::div`]

use std::any::Any;
use std::borrow::Cow;
use wasm_bindgen::{JsCast, UnwrapThrowExt};

use crate::{
    core::{AppendVec, ElementSplice, MessageResult, Mut, View, ViewId, ViewSequence},
    document,
    element_props::ElementProps,
    vec_splice::VecSplice,
    AnyPod, DomNode, DynMessage, Pod, ViewCtx, HTML_NS,
};

mod sealed {
    pub trait Sealed<State, Action, SeqMarker> {}
}

// sealed, because this should only cover `ViewSequences` with the blanket impl below
/// This is basically a specialized dynamically dispatchable [`ViewSequence`], It's currently not able to change the underlying type unlike [`AnyDomView`](crate::AnyDomView), so it should not be used as `dyn DomViewSequence`.
/// It's mostly a hack to avoid a completely static view tree, which unfortunately brings rustc (type-checking) down to its knees and results in long compile-times
pub(crate) trait DomViewSequence<State, Action, SeqMarker>:
    sealed::Sealed<State, Action, SeqMarker> + 'static
{
    /// Get an [`Any`] reference to `self`.
    fn as_any(&self) -> &dyn Any;

    /// Build the associated widgets into `elements` and initialize all states.
    #[must_use]
    fn dyn_seq_build(&self, ctx: &mut ViewCtx, elements: &mut AppendVec<AnyPod>) -> Box<dyn Any>;

    /// Update the associated widgets.
    fn dyn_seq_rebuild(
        &self,
        prev: &dyn DomViewSequence<State, Action, SeqMarker>,
        seq_state: &mut Box<dyn Any>,
        ctx: &mut ViewCtx,
        elements: &mut DomChildrenSplice,
    );

    /// Update the associated widgets.
    fn dyn_seq_teardown(
        &self,
        seq_state: &mut Box<dyn Any>,
        ctx: &mut ViewCtx,
        elements: &mut DomChildrenSplice,
    );

    /// Propagate a message.
    ///
    /// Handle a message, propagating to elements if needed. Here, `id_path` is a slice
    /// of ids, where the first item identifiers a child element of this sequence, if necessary.
    fn dyn_seq_message(
        &self,
        seq_state: &mut Box<dyn Any>,
        id_path: &[ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> MessageResult<Action, DynMessage>;
}

impl<State, Action, SeqMarker, S> sealed::Sealed<State, Action, SeqMarker> for S
where
    State: 'static,
    SeqMarker: 'static,
    Action: 'static,
    S: ViewSequence<State, Action, ViewCtx, AnyPod, SeqMarker, DynMessage>,
{
}

impl<State, Action, SeqMarker, S> DomViewSequence<State, Action, SeqMarker> for S
where
    State: 'static,
    SeqMarker: 'static,
    Action: 'static,
    S: ViewSequence<State, Action, ViewCtx, AnyPod, SeqMarker, DynMessage>,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn dyn_seq_build(&self, ctx: &mut ViewCtx, elements: &mut AppendVec<AnyPod>) -> Box<dyn Any> {
        Box::new(self.seq_build(ctx, elements))
    }

    fn dyn_seq_rebuild(
        &self,
        prev: &dyn DomViewSequence<State, Action, SeqMarker>,
        seq_state: &mut Box<dyn Any>,
        ctx: &mut ViewCtx,
        elements: &mut DomChildrenSplice,
    ) {
        self.seq_rebuild(
            prev.as_any().downcast_ref().unwrap_throw(),
            seq_state.downcast_mut().unwrap_throw(),
            ctx,
            elements,
        );
    }

    fn dyn_seq_teardown(
        &self,
        seq_state: &mut Box<dyn Any>,
        ctx: &mut ViewCtx,
        elements: &mut DomChildrenSplice,
    ) {
        self.seq_teardown(seq_state.downcast_mut().unwrap_throw(), ctx, elements);
    }

    fn dyn_seq_message(
        &self,
        seq_state: &mut Box<dyn Any>,
        id_path: &[ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> MessageResult<Action, DynMessage> {
        self.seq_message(
            seq_state.downcast_mut().unwrap_throw(),
            id_path,
            message,
            app_state,
        )
    }
}

// An alternative idea for this would be to track all the changes (via a `Vec<ChildMutation>`)
// and apply them at once, when this splice is being `Drop`ped, needs some investigation, whether that's better than in place mutations
// TODO maybe we can save some allocations/memory (this needs two extra `Vec`s)
/// This is an [`ElementSplice`] implementation to manage the children of a DOM node in place, it's currently used for updating view sequences
pub struct DomChildrenSplice<'a, 'b, 'c, 'd> {
    scratch: &'a mut AppendVec<AnyPod>,
    children: VecSplice<'b, 'c, AnyPod>,
    ix: usize,
    parent: &'d web_sys::Node,
    parent_was_removed: bool,
}

impl<'a, 'b, 'c, 'd> DomChildrenSplice<'a, 'b, 'c, 'd> {
    pub fn new(
        scratch: &'a mut AppendVec<AnyPod>,
        children: &'b mut Vec<AnyPod>,
        vec_splice_scratch: &'c mut Vec<AnyPod>,
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

impl<'a, 'b, 'c, 'd> ElementSplice<AnyPod> for DomChildrenSplice<'a, 'b, 'c, 'd> {
    fn with_scratch<R>(&mut self, f: impl FnOnce(&mut AppendVec<AnyPod>) -> R) -> R {
        let ret = f(self.scratch);
        for element in self.scratch.drain() {
            self.parent
                .append_child(element.node.as_ref())
                .unwrap_throw();
            self.children.insert(element);
            self.ix += 1;
        }
        ret
    }

    fn insert(&mut self, element: AnyPod) {
        self.parent
            .insert_before(
                element.node.as_ref(),
                self.children.next_mut().map(|p| p.node.as_ref()),
            )
            .unwrap_throw();
        self.ix += 1;
        self.children.insert(element);
    }

    fn mutate<R>(&mut self, f: impl FnOnce(Mut<'_, AnyPod>) -> R) -> R {
        let child = self.children.mutate();
        let ret = f(child.as_mut(self.parent, self.parent_was_removed));
        self.ix += 1;
        ret
    }

    fn skip(&mut self, n: usize) {
        self.children.skip(n);
        self.ix += n;
    }

    fn delete<R>(&mut self, f: impl FnOnce(Mut<'_, AnyPod>) -> R) -> R {
        let mut child = self.children.delete_next();
        let child = child.as_mut(self.parent, true);
        // This is an optimization to avoid too much DOM traffic, otherwise first the children would be deleted from that node in an up-traversal
        if !self.parent_was_removed {
            self.parent.remove_child(child.as_ref()).ok().unwrap_throw();
        }
        f(child)
    }
}

/// Used in all the basic DOM elements as [`View::ViewState`]
pub struct ElementState {
    seq_state: Box<dyn Any>,
    append_scratch: AppendVec<AnyPod>,
    vec_splice_scratch: Vec<AnyPod>,
}

impl ElementState {
    pub fn new(seq_state: Box<dyn Any>) -> Self {
        Self {
            seq_state,
            append_scratch: Default::default(),
            vec_splice_scratch: Default::default(),
        }
    }
}

// These (boilerplatey) functions are there to reduce the boilerplate created by the macro-expansion below.

pub(crate) fn build_element<State, Action, Element, SeqMarker>(
    children: &dyn DomViewSequence<State, Action, SeqMarker>,
    tag_name: &str,
    ns: &str,
    ctx: &mut ViewCtx,
) -> (Element, ElementState)
where
    State: 'static,
    Action: 'static,
    Element: 'static,
    SeqMarker: 'static,
    Element: From<Pod<web_sys::Element, ElementProps>>,
{
    let mut elements = AppendVec::default();
    let state = ElementState::new(children.dyn_seq_build(ctx, &mut elements));
    (
        Pod::new_element(elements.into_inner(), ns, tag_name).into(),
        state,
    )
}

pub(crate) fn rebuild_element<'el, State, Action, Element, SeqMarker>(
    children: &dyn DomViewSequence<State, Action, SeqMarker>,
    prev_children: &dyn DomViewSequence<State, Action, SeqMarker>,
    element: Mut<'el, Pod<Element, ElementProps>>,
    state: &mut ElementState,
    ctx: &mut ViewCtx,
) -> Mut<'el, Pod<Element, ElementProps>>
where
    State: 'static,
    Action: 'static,
    Element: 'static,
    SeqMarker: 'static,
    Element: DomNode<ElementProps>,
{
    let mut dom_children_splice = DomChildrenSplice::new(
        &mut state.append_scratch,
        &mut element.props.children,
        &mut state.vec_splice_scratch,
        element.node.as_ref(),
        element.was_removed,
    );
    children.dyn_seq_rebuild(
        prev_children,
        &mut state.seq_state,
        ctx,
        &mut dom_children_splice,
    );
    element
}

pub(crate) fn teardown_element<State, Action, Element, SeqMarker>(
    children: &dyn DomViewSequence<State, Action, SeqMarker>,
    element: Mut<'_, Pod<Element, ElementProps>>,
    state: &mut ElementState,
    ctx: &mut ViewCtx,
) where
    State: 'static,
    Action: 'static,
    Element: 'static,
    SeqMarker: 'static,
    Element: DomNode<ElementProps>,
{
    let mut dom_children_splice = DomChildrenSplice::new(
        &mut state.append_scratch,
        &mut element.props.children,
        &mut state.vec_splice_scratch,
        element.node.as_ref(),
        true,
    );
    children.dyn_seq_teardown(&mut state.seq_state, ctx, &mut dom_children_splice);
}

/// An element that can change its tag, it's useful for autonomous custom elements (i.e. web components)
pub struct CustomElement<State, Action, SeqMarker> {
    name: Cow<'static, str>,
    children: Box<dyn DomViewSequence<State, Action, SeqMarker>>,
}

/// An element that can change its tag, it's useful for autonomous custom elements (i.e. web components)
pub fn custom_element<State, Action, SeqMarker, Children>(
    name: impl Into<Cow<'static, str>>,
    children: Children,
) -> CustomElement<State, Action, SeqMarker>
where
    State: 'static,
    Action: 'static,
    SeqMarker: 'static,
    Children: ViewSequence<State, Action, ViewCtx, AnyPod, SeqMarker, DynMessage>,
{
    CustomElement {
        name: name.into(),
        children: Box::new(children),
    }
}

impl<State, Action, SeqMarker> View<State, Action, ViewCtx, DynMessage>
    for CustomElement<State, Action, SeqMarker>
where
    State: 'static,
    Action: 'static,
    SeqMarker: 'static,
{
    type Element = Pod<web_sys::HtmlElement, ElementProps>;

    type ViewState = ElementState;

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        build_element(&*self.children, &self.name, HTML_NS, ctx)
    }

    fn rebuild<'el>(
        &self,
        prev: &Self,
        element_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<'el, Self::Element>,
    ) -> Mut<'el, Self::Element> {
        if prev.name != self.name {
            let new_element = document()
                .create_element_ns(Some(HTML_NS), &self.name)
                .unwrap_throw();

            while let Some(child) = element.node.child_nodes().get(0) {
                new_element.append_child(&child).unwrap_throw();
            }
            element
                .parent
                .replace_child(&new_element, element.node)
                .unwrap_throw();
            *element.node = new_element.dyn_into().unwrap_throw();
        }

        rebuild_element(
            &*self.children,
            &*prev.children,
            element,
            element_state,
            ctx,
        )
    }

    fn teardown(
        &self,
        element_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<'_, Self::Element>,
    ) {
        teardown_element(&*self.children, element, element_state, ctx);
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> MessageResult<Action, DynMessage> {
        self.children
            .dyn_seq_message(&mut view_state.seq_state, id_path, message, app_state)
    }
}

macro_rules! define_element {
    ($ns:expr, ($ty_name:ident, $name:ident, $dom_interface:ident)) => {
        define_element!($ns, ($ty_name, $name, $dom_interface, stringify!($name)));
    };
    ($ns:expr, ($ty_name:ident, $name:ident, $dom_interface:ident, $tag_name:expr)) => {
        pub struct $ty_name<State, Action, SeqMarker> {
            children: Box<dyn DomViewSequence<State, Action, SeqMarker>>,
        }

        /// Builder function for a
        #[doc = concat!("`", $tag_name, "`")]
        /// element view.
        pub fn $name<
            State: 'static,
            Action: 'static,
            SeqMarker: 'static,
            Children: ViewSequence<State, Action, ViewCtx, AnyPod, SeqMarker, DynMessage>,
        >(
            children: Children,
        ) -> $ty_name<State, Action, SeqMarker> {
            $ty_name {
                children: Box::new(children),
            }
        }

        impl<State, Action, SeqMarker> View<State, Action, ViewCtx, DynMessage>
            for $ty_name<State, Action, SeqMarker>
        where
            State: 'static,
            Action: 'static,
            SeqMarker: 'static,
        {
            type Element = Pod<web_sys::$dom_interface, ElementProps>;

            type ViewState = ElementState;

            fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
                build_element(&*self.children, $tag_name, $ns, ctx)
            }

            fn rebuild<'el>(
                &self,
                prev: &Self,
                element_state: &mut Self::ViewState,
                ctx: &mut ViewCtx,
                element: Mut<'el, Self::Element>,
            ) -> Mut<'el, Self::Element> {
                rebuild_element(
                    &*self.children,
                    &*prev.children,
                    element,
                    element_state,
                    ctx,
                )
            }

            fn teardown(
                &self,
                element_state: &mut Self::ViewState,
                ctx: &mut ViewCtx,
                element: Mut<'_, Self::Element>,
            ) {
                teardown_element(&*self.children, element, element_state, ctx);
            }

            fn message(
                &self,
                view_state: &mut Self::ViewState,
                id_path: &[ViewId],
                message: DynMessage,
                app_state: &mut State,
            ) -> MessageResult<Action, DynMessage> {
                self.children.dyn_seq_message(
                    &mut view_state.seq_state,
                    id_path,
                    message,
                    app_state,
                )
            }
        }
    };
}

macro_rules! define_elements {
    ($ns:ident, $($element_def:tt,)*) => {
        use super::{build_element, rebuild_element, teardown_element, DomViewSequence, ElementState};
        use crate::{
            core::{MessageResult, Mut, ViewId, ViewSequence},
            AnyPod, DynMessage, ElementProps, Pod, View, ViewCtx,
        };
        $(define_element!(crate::$ns, $element_def);)*
    };
}

pub mod html {
    //! HTML elements with the namespace [`HTML_NS`](`crate::HTML_NS`)
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
        (A, a, HtmlAnchorElement),
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
    //! MathML elements with the namespace [`MATHML_NS`](`crate::MATHML_NS`)
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
    //! SVG elements with the namespace [`SVG_NS`](`crate::SVG_NS`)
    define_elements!(
        SVG_NS,
        (Svg, svg, SvgsvgElement),
        (A, a, SvgaElement),
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
