//! Types that wrap [`Element`][super::Element] and represent specific element types.
//!
macro_rules! elements {
    () => {};
    (($ty_name:ident, $builder_name:ident, $name:literal, $web_sys_ty:ty), $($rest:tt)*) => {
        element!($ty_name, $builder_name, $name, $web_sys_ty);
        elements!($($rest)*);
    };
}

macro_rules! element {
    ($ty_name:ident, $builder_name:ident, $name:literal, $web_sys_ty:ty) => {
        /// A view representing a
        #[doc = concat!("`", $name, "`")]
        /// element.
        pub struct $ty_name<ViewSeq>(crate::Element<$web_sys_ty, ViewSeq>);

        /// Builder function for a
        #[doc = concat!("`", $name, "`")]
        /// view.
        pub fn $builder_name<ViewSeq>(children: ViewSeq) -> $ty_name<ViewSeq> {
            $ty_name(crate::element($name, children))
        }

        impl<ViewSeq> $ty_name<ViewSeq> {
            /// Set an attribute on this element.
            ///
            /// # Panics
            ///
            /// If the name contains characters that are not valid in an attribute name,
            /// then the `View::build`/`View::rebuild` functions will panic for this view.
            pub fn attr(
                mut self,
                name: impl Into<std::borrow::Cow<'static, str>>,
                value: impl Into<std::borrow::Cow<'static, str>>,
            ) -> Self {
                self.0.set_attr(name, value);
                self
            }

            /// Set an attribute on this element.
            ///
            /// # Panics
            ///
            /// If the name contains characters that are not valid in an attribute name,
            /// then the `View::build`/`View::rebuild` functions will panic for this view.
            pub fn set_attr(
                &mut self,
                name: impl Into<std::borrow::Cow<'static, str>>,
                value: impl Into<std::borrow::Cow<'static, str>>,
            ) -> &mut Self {
                self.0.set_attr(name, value);
                self
            }
        }

        impl<ViewSeq> crate::view::ViewMarker for $ty_name<ViewSeq> {}

        impl<T_, A_, ViewSeq> crate::view::View<T_, A_> for $ty_name<ViewSeq>
        where
            ViewSeq: crate::view::ViewSequence<T_, A_>,
        {
            type State = crate::ElementState<ViewSeq::State>;
            type Element = $web_sys_ty;

            fn build(
                &self,
                cx: &mut crate::context::Cx,
            ) -> (xilem_core::Id, Self::State, Self::Element) {
                self.0.build(cx)
            }

            fn rebuild(
                &self,
                cx: &mut crate::context::Cx,
                prev: &Self,
                id: &mut xilem_core::Id,
                state: &mut Self::State,
                element: &mut Self::Element,
            ) -> crate::ChangeFlags {
                self.0.rebuild(cx, &prev.0, id, state, element)
            }

            fn message(
                &self,
                id_path: &[xilem_core::Id],
                state: &mut Self::State,
                message: Box<dyn std::any::Any>,
                app_state: &mut T_,
            ) -> xilem_core::MessageResult<A_> {
                self.0.message(id_path, state, message, app_state)
            }
        }
    };
}

// void elements (those without children) are `area`, `base`, `br`, `col`,
// `embed`, `hr`, `img`, `input`, `link`, `meta`, `source`, `track`, `wbr`
elements!(
    // the order is copied from
    // https://developer.mozilla.org/en-US/docs/Web/HTML/Element
    // DOM interfaces copied from https://html.spec.whatwg.org/multipage/grouping-content.html and friends

    // content sectioning
    (Address, address, "address", web_sys::HtmlElement),
    (Article, article, "article", web_sys::HtmlElement),
    (Aside, aside, "aside", web_sys::HtmlElement),
    (Footer, footer, "footer", web_sys::HtmlElement),
    (Header, header, "header", web_sys::HtmlElement),
    (H1, h1, "h1", web_sys::HtmlHeadingElement),
    (H2, h2, "h2", web_sys::HtmlHeadingElement),
    (H3, h3, "h3", web_sys::HtmlHeadingElement),
    (H4, h4, "h4", web_sys::HtmlHeadingElement),
    (H5, h5, "h5", web_sys::HtmlHeadingElement),
    (H6, h6, "h6", web_sys::HtmlHeadingElement),
    (Hgroup, hgroup, "hgroup", web_sys::HtmlElement),
    (Main, main, "main", web_sys::HtmlElement),
    (Nav, nav, "nav", web_sys::HtmlElement),
    (Section, section, "section", web_sys::HtmlElement),
    // text content
    (
        Blockquote,
        blockquote,
        "blockquote",
        web_sys::HtmlQuoteElement
    ),
    (Dd, dd, "dd", web_sys::HtmlElement),
    (Div, div, "div", web_sys::HtmlDivElement),
    (Dl, dl, "dl", web_sys::HtmlDListElement),
    (Dt, dt, "dt", web_sys::HtmlElement),
    (Figcaption, figcaption, "figcaption", web_sys::HtmlElement),
    (Figure, figure, "figure", web_sys::HtmlElement),
    (Hr, hr, "hr", web_sys::HtmlHrElement),
    (Li, li, "li", web_sys::HtmlLiElement),
    (Menu, menu, "menu", web_sys::HtmlMenuElement),
    (Ol, ol, "ol", web_sys::HtmlOListElement),
    (P, p, "p", web_sys::HtmlParagraphElement),
    (Pre, pre, "pre", web_sys::HtmlPreElement),
    (Ul, ul, "ul", web_sys::HtmlUListElement),
    // inline text
    (A, a, "a", web_sys::HtmlAnchorElement),
    (Abbr, abbr, "abbr", web_sys::HtmlElement),
    (B, b, "b", web_sys::HtmlElement),
    (Bdi, bdi, "bdi", web_sys::HtmlElement),
    (Bdo, bdo, "bdo", web_sys::HtmlElement),
    (Br, br, "br", web_sys::HtmlBrElement),
    (Cite, cite, "cite", web_sys::HtmlElement),
    (Code, code, "code", web_sys::HtmlElement),
    (Data, data, "data", web_sys::HtmlDataElement),
    (Dfn, dfn, "dfn", web_sys::HtmlElement),
    (Em, em, "em", web_sys::HtmlElement),
    (I, i, "i", web_sys::HtmlElement),
    (Kbd, kbd, "kbd", web_sys::HtmlElement),
    (Mark, mark, "mark", web_sys::HtmlElement),
    (Q, q, "q", web_sys::HtmlQuoteElement),
    (Rp, rp, "rp", web_sys::HtmlElement),
    (Rt, rt, "rt", web_sys::HtmlElement),
    (Ruby, ruby, "ruby", web_sys::HtmlElement),
    (S, s, "s", web_sys::HtmlElement),
    (Samp, samp, "samp", web_sys::HtmlElement),
    (Small, small, "small", web_sys::HtmlElement),
    (Span, span, "span", web_sys::HtmlSpanElement),
    (Strong, strong, "strong", web_sys::HtmlElement),
    (Sub, sub, "sub", web_sys::HtmlElement),
    (Sup, sup, "sup", web_sys::HtmlElement),
    (Time, time, "time", web_sys::HtmlTimeElement),
    (U, u, "u", web_sys::HtmlElement),
    (Var, var, "var", web_sys::HtmlElement),
    (Wbr, wbr, "wbr", web_sys::HtmlElement),
    // image and multimedia
    (Area, area, "area", web_sys::HtmlAreaElement),
    (Audio, audio, "audio", web_sys::HtmlAudioElement),
    (Img, img, "img", web_sys::HtmlImageElement),
    (Map, map, "map", web_sys::HtmlMapElement),
    (Track, track, "track", web_sys::HtmlTrackElement),
    (Video, video, "video", web_sys::HtmlVideoElement),
    // embedded content
    (Embed, embed, "embed", web_sys::HtmlEmbedElement),
    (Iframe, iframe, "iframe", web_sys::HtmlIFrameElement),
    (Object, object, "object", web_sys::HtmlObjectElement),
    (Picture, picture, "picture", web_sys::HtmlPictureElement),
    (Portal, portal, "portal", web_sys::HtmlElement),
    (Source, source, "source", web_sys::HtmlSourceElement),
    // SVG and MathML (TODO, svg and mathml elements)
    (Svg, svg, "svg", web_sys::HtmlElement),
    (Math, math, "math", web_sys::HtmlElement),
    // scripting
    (Canvas, canvas, "canvas", web_sys::HtmlCanvasElement),
    (Noscript, noscript, "noscript", web_sys::HtmlElement),
    (Script, script, "script", web_sys::HtmlScriptElement),
    // demarcating edits
    (Del, del, "del", web_sys::HtmlModElement),
    (Ins, ins, "ins", web_sys::HtmlModElement),
    // tables
    (
        Caption,
        caption,
        "caption",
        web_sys::HtmlTableCaptionElement
    ),
    (Col, col, "col", web_sys::HtmlTableColElement),
    (Colgroup, colgroup, "colgroup", web_sys::HtmlTableColElement),
    (Table, table, "table", web_sys::HtmlTableSectionElement),
    (Tbody, tbody, "tbody", web_sys::HtmlTableSectionElement),
    (Td, td, "td", web_sys::HtmlTableCellElement),
    (Tfoot, tfoot, "tfoot", web_sys::HtmlTableSectionElement),
    (Th, th, "th", web_sys::HtmlTableCellElement),
    (Thead, thead, "thead", web_sys::HtmlTableSectionElement),
    (Tr, tr, "tr", web_sys::HtmlTableRowElement),
    // forms
    (Button, button, "button", web_sys::HtmlButtonElement),
    (Datalist, datalist, "datalist", web_sys::HtmlDataListElement),
    (Fieldset, fieldset, "fieldset", web_sys::HtmlFieldSetElement),
    (Form, form, "form", web_sys::HtmlFormElement),
    (Input, input, "input", web_sys::HtmlInputElement),
    (Label, label, "label", web_sys::HtmlLabelElement),
    (Legend, legend, "legend", web_sys::HtmlLegendElement),
    (Meter, meter, "meter", web_sys::HtmlMeterElement),
    (Optgroup, optgroup, "optgroup", web_sys::HtmlOptGroupElement),
    (Option, option, "option", web_sys::HtmlOptionElement),
    (Output, output, "output", web_sys::HtmlOutputElement),
    (Progress, progress, "progress", web_sys::HtmlProgressElement),
    (Select, select, "select", web_sys::HtmlSelectElement),
    (Textarea, textarea, "textarea", web_sys::HtmlTextAreaElement),
    // interactive elements,
    (Details, details, "details", web_sys::HtmlDetailsElement),
    (Dialog, dialog, "dialog", web_sys::HtmlDialogElement),
    (Summary, summary, "summary", web_sys::HtmlElement),
    // web components,
    (Slot, slot, "slot", web_sys::HtmlSlotElement),
    (Template, template, "template", web_sys::HtmlTemplateElement),
);
