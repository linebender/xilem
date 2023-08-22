use wasm_bindgen::JsCast;

use crate::{
    dom::{attribute::Attr, event::EventListener, interfaces::Element},
    OptionalAction,
};

macro_rules! dom_interface_trait_definitions {
    ($($dom_interface:ident : $super_dom_interface: ident $body: tt),*) => {
        $(
            pub trait $dom_interface<T, A = ()>: $super_dom_interface<T, A> $body

            impl<T, A, E: $dom_interface<T, A>> $dom_interface<T, A> for Attr<E> { }

            impl<T, A, E, Ev, F, OA> $dom_interface<T, A> for EventListener<E, Ev, F>
            where
                F: Fn(&mut T, Ev) -> OA,
                E: $dom_interface<T, A>,
                Ev: JsCast + 'static,
                OA: OptionalAction<A>,
            {
            }
        )*
    };
}

dom_interface_trait_definitions!(
    HtmlAnchorElement : HtmlElement {},
    HtmlAreaElement : HtmlElement {},
    HtmlAudioElement : HtmlMediaElement {},
    HtmlBaseElement : HtmlElement {},
    HtmlBodyElement : HtmlElement {},
    HtmlBrElement : HtmlElement {},
    HtmlButtonElement : HtmlElement {},
    HtmlCanvasElement : HtmlElement {
        // Basic idea how to get strong typed attributes working
        // Rather the DOM interface attributes than HTML/XML attributes though, as they are (mostly) well defined by the spec,
        // compared to HTML/XML attributes.
        fn width(self, width: u32) -> Attr<Self> {
            self.attr("width", width)
        }
        fn height(self, height: u32) -> Attr<Self> {
            self.attr("height", height)
        }
    },
    HtmlDataElement : HtmlElement {},
    HtmlDataListElement : HtmlElement {},
    HtmlDetailsElement : HtmlElement {},
    HtmlDialogElement : HtmlElement {},
    HtmlDirectoryElement : HtmlElement {},
    HtmlDivElement : HtmlElement {},
    HtmlDListElement : HtmlElement {},
    HtmlElement : Element {},
    HtmlUnknownElement : HtmlElement {},
    HtmlEmbedElement : HtmlElement {},
    HtmlFieldSetElement : HtmlElement {},
    HtmlFontElement : HtmlElement {},
    HtmlFormElement : HtmlElement {},
    HtmlFrameElement : HtmlElement {},
    HtmlFrameSetElement : HtmlElement {},
    HtmlHeadElement : HtmlElement {},
    HtmlHeadingElement : HtmlElement {},
    HtmlHrElement : HtmlElement {},
    HtmlHtmlElement : HtmlElement {},
    HtmlIFrameElement : HtmlElement {},
    HtmlImageElement : HtmlElement {},
    HtmlInputElement : HtmlElement {},
    HtmlLabelElement : HtmlElement {},
    HtmlLegendElement : HtmlElement {},
    HtmlLiElement : HtmlElement {},
    HtmlLinkElement : HtmlElement {},
    HtmlMapElement : HtmlElement {},
    HtmlMediaElement : HtmlElement {},
    HtmlMenuElement : HtmlElement {},
    HtmlMenuItemElement : HtmlElement {},
    HtmlMetaElement : HtmlElement {},
    HtmlMeterElement : HtmlElement {},
    HtmlModElement : HtmlElement {},
    HtmlObjectElement : HtmlElement {},
    HtmlOListElement : HtmlElement {},
    HtmlOptGroupElement : HtmlElement {},
    HtmlOptionElement : HtmlElement {},
    HtmlOutputElement : HtmlElement {},
    HtmlParagraphElement : HtmlElement {},
    HtmlParamElement : HtmlElement {},
    HtmlPictureElement : HtmlElement {},
    HtmlPreElement : HtmlElement {},
    HtmlProgressElement : HtmlElement {},
    HtmlQuoteElement : HtmlElement {},
    HtmlScriptElement : HtmlElement {},
    HtmlSelectElement : HtmlElement {},
    HtmlSlotElement : HtmlElement {},
    HtmlSourceElement : HtmlElement {},
    HtmlSpanElement : HtmlElement {},
    HtmlStyleElement : HtmlElement {},
    HtmlTableCaptionElement : HtmlElement {},
    HtmlTableCellElement : HtmlElement {},
    HtmlTableColElement : HtmlElement {},
    HtmlTableElement : HtmlElement {},
    HtmlTableRowElement : HtmlElement {},
    HtmlTableSectionElement : HtmlElement {},
    HtmlTemplateElement : HtmlElement {},
    HtmlTimeElement : HtmlElement {},
    HtmlTextAreaElement : HtmlElement {},
    HtmlTitleElement : HtmlElement {},
    HtmlTrackElement : HtmlElement {},
    HtmlUListElement : HtmlElement {},
    HtmlVideoElement : HtmlMediaElement {}
);
