//! Types that wrap [`Element`][super::Element] and represent specific element types.
//!
macro_rules! elements {
    () => {};
    (($ty_name:ident, $builder_name:ident, $web_sys_ty:ty), $($rest:tt)*) => {
        element!($ty_name, $builder_name, $web_sys_ty);
        elements!($($rest)*);
    };
}

macro_rules! element {
    ($ty_name:ident, $builder_name:ident, $web_sys_ty:ty) => {
        /// Tag for a
        #[doc = concat!("`", stringify!($builder_name), "`")]
        /// view.
        pub struct $ty_name;

        impl $crate::element::ElementTag for $ty_name {
            type WebSysElement = $web_sys_ty;
            fn name() -> &'static str {
                stringify!($builder_name)
            }
        }

        /// Builder function for a
        #[doc = concat!("`", stringify!($builder_name), "`")]
        /// view.
        pub fn $builder_name<ViewSeq>(children: ViewSeq) -> $crate::Element<$ty_name, ViewSeq> {
            crate::element(stringify!($builder_name), children)
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
    (Address, address, web_sys::HtmlElement),
    (Article, article, web_sys::HtmlElement),
    (Aside, aside, web_sys::HtmlElement),
    (Footer, footer, web_sys::HtmlElement),
    (Header, header, web_sys::HtmlElement),
    (H1, h1, web_sys::HtmlHeadingElement),
    (H2, h2, web_sys::HtmlHeadingElement),
    (H3, h3, web_sys::HtmlHeadingElement),
    (H4, h4, web_sys::HtmlHeadingElement),
    (H5, h5, web_sys::HtmlHeadingElement),
    (H6, h6, web_sys::HtmlHeadingElement),
    (Hgroup, hgroup, web_sys::HtmlElement),
    (Main, main, web_sys::HtmlElement),
    (Nav, nav, web_sys::HtmlElement),
    (Section, section, web_sys::HtmlElement),
    // text content
    (Blockquote, blockquote, web_sys::HtmlQuoteElement),
    (Dd, dd, web_sys::HtmlElement),
    (Div, div, web_sys::HtmlDivElement),
    (Dl, dl, web_sys::HtmlDListElement),
    (Dt, dt, web_sys::HtmlElement),
    (Figcaption, figcaption, web_sys::HtmlElement),
    (Figure, figure, web_sys::HtmlElement),
    (Hr, hr, web_sys::HtmlHrElement),
    (Li, li, web_sys::HtmlLiElement),
    (Menu, menu, web_sys::HtmlMenuElement),
    (Ol, ol, web_sys::HtmlOListElement),
    (P, p, web_sys::HtmlParagraphElement),
    (Pre, pre, web_sys::HtmlPreElement),
    (Ul, ul, web_sys::HtmlUListElement),
    // inline text
    (A, a, web_sys::HtmlAnchorElement),
    (Abbr, abbr, web_sys::HtmlElement),
    (B, b, web_sys::HtmlElement),
    (Bdi, bdi, web_sys::HtmlElement),
    (Bdo, bdo, web_sys::HtmlElement),
    (Br, br, web_sys::HtmlBrElement),
    (Cite, cite, web_sys::HtmlElement),
    (Code, code, web_sys::HtmlElement),
    (Data, data, web_sys::HtmlDataElement),
    (Dfn, dfn, web_sys::HtmlElement),
    (Em, em, web_sys::HtmlElement),
    (I, i, web_sys::HtmlElement),
    (Kbd, kbd, web_sys::HtmlElement),
    (Mark, mark, web_sys::HtmlElement),
    (Q, q, web_sys::HtmlQuoteElement),
    (Rp, rp, web_sys::HtmlElement),
    (Rt, rt, web_sys::HtmlElement),
    (Ruby, ruby, web_sys::HtmlElement),
    (S, s, web_sys::HtmlElement),
    (Samp, samp, web_sys::HtmlElement),
    (Small, small, web_sys::HtmlElement),
    (Span, span, web_sys::HtmlSpanElement),
    (Strong, strong, web_sys::HtmlElement),
    (Sub, sub, web_sys::HtmlElement),
    (Sup, sup, web_sys::HtmlElement),
    (Time, time, web_sys::HtmlTimeElement),
    (U, u, web_sys::HtmlElement),
    (Var, var, web_sys::HtmlElement),
    (Wbr, wbr, web_sys::HtmlElement),
    // image and multimedia
    (Area, area, web_sys::HtmlAreaElement),
    (Audio, audio, web_sys::HtmlAudioElement),
    (Img, img, web_sys::HtmlImageElement),
    (Map, map, web_sys::HtmlMapElement),
    (Track, track, web_sys::HtmlTrackElement),
    (Video, video, web_sys::HtmlVideoElement),
    // embedded content
    (Embed, embed, web_sys::HtmlEmbedElement),
    (Iframe, iframe, web_sys::HtmlIFrameElement),
    (Object, object, web_sys::HtmlObjectElement),
    (Picture, picture, web_sys::HtmlPictureElement),
    (Portal, portal, web_sys::HtmlElement),
    (Source, source, web_sys::HtmlSourceElement),
    // SVG and MathML (TODO, svg and mathml elements)
    (Svg, svg, web_sys::HtmlElement),
    (Math, math, web_sys::HtmlElement),
    // scripting
    (Canvas, canvas, web_sys::HtmlCanvasElement),
    (Noscript, noscript, web_sys::HtmlElement),
    (Script, script, web_sys::HtmlScriptElement),
    // demarcating edits
    (Del, del, web_sys::HtmlModElement),
    (Ins, ins, web_sys::HtmlModElement),
    // tables
    (Caption, caption, web_sys::HtmlTableCaptionElement),
    (Col, col, web_sys::HtmlTableColElement),
    (Colgroup, colgroup, web_sys::HtmlTableColElement),
    (Table, table, web_sys::HtmlTableSectionElement),
    (Tbody, tbody, web_sys::HtmlTableSectionElement),
    (Td, td, web_sys::HtmlTableCellElement),
    (Tfoot, tfoot, web_sys::HtmlTableSectionElement),
    (Th, th, web_sys::HtmlTableCellElement),
    (Thead, thead, web_sys::HtmlTableSectionElement),
    (Tr, tr, web_sys::HtmlTableRowElement),
    // forms
    (Button, button, web_sys::HtmlButtonElement),
    (Datalist, datalist, web_sys::HtmlDataListElement),
    (Fieldset, fieldset, web_sys::HtmlFieldSetElement),
    (Form, form, web_sys::HtmlFormElement),
    (Input, input, web_sys::HtmlInputElement),
    (Label, label, web_sys::HtmlLabelElement),
    (Legend, legend, web_sys::HtmlLegendElement),
    (Meter, meter, web_sys::HtmlMeterElement),
    (Optgroup, optgroup, web_sys::HtmlOptGroupElement),
    (Option, option, web_sys::HtmlOptionElement),
    (Output, output, web_sys::HtmlOutputElement),
    (Progress, progress, web_sys::HtmlProgressElement),
    (Select, select, web_sys::HtmlSelectElement),
    (Textarea, textarea, web_sys::HtmlTextAreaElement),
    // interactive elements,
    (Details, details, web_sys::HtmlDetailsElement),
    (Dialog, dialog, web_sys::HtmlDialogElement),
    (Summary, summary, web_sys::HtmlElement),
    // web components,
    (Slot, slot, web_sys::HtmlSlotElement),
    (Template, template, web_sys::HtmlTemplateElement),
);
