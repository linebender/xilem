// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

#[derive(Debug)]
enum TagCloseBehaviour {
    /// Nothing needs to happen when the span is closed (i.e. we didn't do anything for it?)
    None,
    /// A `span.invisible` is ending (and so `emit` should be toggled back on)
    Hidden,
    /// An ellipisis should be output at the end of this item.
    Ellipsis,
    /// A paragraph.
    Paragraph,
}

/// Convert sanitised HTML into a displayable string.
///
/// See <https://docs.joinmastodon.org/spec/activitypub/#sanitization> for the tags we have to support.
///
/// Note:
/// 1) We convert HTML entities to their regular value (hopefully?)
/// 2) We only handle the `p`, `br`, `span.invisible`, `span.ellipsis` cases
/// 3) We don't handle `microformat` at all.
// TODO: We know this code is not great (and probably way too imperative!)
// We're deferring refactoring this until we want to handle more attributes.
pub(crate) fn handle_content_html(content: &str) -> String {
    let _span = tracing::info_span!("handle_content_html").entered();
    let tokeniser = html5gum::Tokenizer::new(content);
    // The resulting string will *always*(?) be shorter than the initial string
    let mut result = String::with_capacity(content.len());

    let mut stack = Vec::<TagCloseBehaviour>::new();
    let mut emit = true;
    // Tokeniser here returns Result<Token, !>, so this is an infallible pattern.
    for Ok(token) in tokeniser {
        match token {
            html5gum::Token::StartTag(mut start_tag) => match start_tag.name.as_slice() {
                b"p" => {
                    if !start_tag.attributes.is_empty() {
                        tracing::warn!(
                            "Got unexpected attributes for <p>: {:?}.",
                            start_tag.attributes
                        );
                    }
                    if start_tag.self_closing {
                        tracing::warn!("Got unexpected self-closing paragraph.");
                        result.push_str("\n\n");
                    } else {
                        stack.push(TagCloseBehaviour::Paragraph);
                    }
                }
                b"br" => {
                    result.push('\n');
                    if !start_tag.attributes.is_empty() {
                        tracing::warn!(
                            "Got unexpected attributes for <br>: {:?}.",
                            start_tag.attributes
                        );
                    }
                    if !start_tag.self_closing {
                        tracing::error!("Expected <br/> to be self closing.");
                        stack.push(TagCloseBehaviour::None);
                    }
                }
                b"span" => {
                    if let Some(class) = start_tag.attributes.remove(b"class".as_slice()) {
                        let class_string = String::from_utf8(class.0).unwrap();
                        let mut has_ellipsis = false;
                        let mut has_invisible = false;
                        for class in class_string.split_whitespace() {
                            if class == "ellipsis" {
                                has_ellipsis = true;
                            } else if class == "invisible" {
                                has_invisible = true;
                            } else {
                                tracing::warn!("Unhandled span class {class:?}.");
                            }
                        }
                        if start_tag.self_closing {
                            if has_ellipsis {
                                tracing::warn!("Got unexpectedly empty ellipsis span.");
                                result.push_str("...");
                            } else {
                                tracing::warn!("Got unexpectedly empty span.");
                            }
                        } else {
                            match (has_ellipsis, has_invisible) {
                                (true, true) => {
                                    tracing::error!(
                                        "Unexpected mixing of invisible and ellipsis classes."
                                    );
                                    stack.push(TagCloseBehaviour::None);
                                }
                                (true, false) => {
                                    stack.push(TagCloseBehaviour::Ellipsis);
                                }
                                (false, true) => {
                                    emit = false;
                                    stack.push(TagCloseBehaviour::Hidden);
                                }
                                (false, false) => {
                                    stack.push(TagCloseBehaviour::None);
                                }
                            }
                        }
                    } else if !start_tag.self_closing {
                        stack.push(TagCloseBehaviour::None);
                    }
                }
                b"a" => {
                    tracing::info!("Encountered intentionally unhandled <a> tag.");
                    if !start_tag.self_closing {
                        stack.push(TagCloseBehaviour::None);
                    }
                }
                _ => {
                    tracing::error!("Unhandled tag <{:?}>", start_tag.name);
                    if !start_tag.self_closing {
                        stack.push(TagCloseBehaviour::None);
                    }
                }
            },
            html5gum::Token::EndTag(end_tag) => match stack.pop() {
                Some(action) => match action {
                    TagCloseBehaviour::None => {}
                    TagCloseBehaviour::Hidden => {
                        if emit {
                            tracing::error!("Nested `invisible` spans closed by {end_tag:?}.");
                        }
                        emit = true;
                    }
                    TagCloseBehaviour::Ellipsis => {
                        result.push_str("...");
                    }
                    TagCloseBehaviour::Paragraph => result.push_str("\n\n"),
                },
                None => {
                    let end_tag = end_tag.name.as_slice();
                    panic!(
                        "Got unexpected extra closing tag {end_tag:?} ({end_tag:p}) in {content} ({content:p})."
                    );
                }
            },
            html5gum::Token::String(html_string) => {
                if emit {
                    result.push_str(
                        String::from_utf8(html_string.0)
                            .expect("utf-8 input implies utf-8 output.")
                            .as_str(),
                    );
                }
            }
            html5gum::Token::Comment(html_string) => {
                tracing::warn!("Got HTML comment in post {html_string:?}; this is unexpected.");
            }
            html5gum::Token::Doctype(doctype) => {
                tracing::error!("Got doctype in post {doctype:?}; this is unexpected.");
            }
            html5gum::Token::Error(error) => {
                tracing::error!("Got error token parsing post:\n{error}.");
            }
        }
    }
    result
}
