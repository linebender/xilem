// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

#[derive(Debug)]
enum TagCloseBehaviour {
    /// Nothing needs to happen when the span is closed (i.e. we didn't do anything for it?)
    None,
    /// A `span.invisible` is ending (and so `emit` should be toggled back on)
    Hidden,
    /// An ellipsis should be output at the end of this item.
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
///
/// For certain error cases, this [`warn`](tracing::warn)s (or `error`s).
/// For additional context, the app can be run with the environment variable
/// `RUST_LOG` set to `"info,placehero::html_content=trace"`.
// TODO: We know this code is not great (and probably way too imperative!)
// We're deferring refactoring this until we want to handle more attributes.
pub(crate) fn status_html_to_plaintext(content: &str) -> String {
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
                    // `br` is empty, so doesn't need handling of it closing.
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
                                tracing::warn!(?class, "Unhandled span class.");
                                tracing::trace!(
                                    ?class,
                                    status = content,
                                    "Context for unhandled span class."
                                );
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
                                    tracing::trace!(
                                        status = content,
                                        "Context for invisible ellipsis."
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
                    tracing::trace!("Encountered intentionally unhandled <a> tag.");
                    if !start_tag.self_closing {
                        stack.push(TagCloseBehaviour::None);
                    }
                }
                _ => {
                    let tag_value = String::from_utf8_lossy(&start_tag.name);
                    tracing::error!(tag = format_args!("<{:?}>", tag_value), "Unhandled tag.");
                    tracing::trace!(
                        tag = format_args!("<{:?}>", tag_value),
                        status = content,
                        "Context for unhandled tag."
                    );
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
                            tracing::error!(
                                end_tag = format_args!("</{:?}>", end_tag.name),
                                "Nested `invisible` spans."
                            );
                            tracing::trace!(
                                end_tag = format_args!("</{:?}>", end_tag.name),
                                status = content,
                                "Context for nested `invisible` spans."
                            );
                        }
                        emit = true;
                    }
                    TagCloseBehaviour::Ellipsis => {
                        result.push_str("...");
                    }
                    TagCloseBehaviour::Paragraph => result.push_str("\n\n"),
                },
                None => {
                    tracing::error!(
                        end_tag = format_args!("</{:?}>", end_tag.name),
                        "Got unexpected extra closing tag."
                    );
                    tracing::trace!(
                        end_tag = format_args!("</{:?}>", end_tag.name),
                        status = content,
                        "Context for unexpected extra closing tag."
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
                tracing::error!(?doctype, "Unexpected doctype in post.");
                tracing::trace!(
                    ?doctype,
                    status = content,
                    "Context for unexpected doctype."
                );
            }
            html5gum::Token::Error(error) => {
                tracing::error!(?error, "Got error parsing html token.");
                tracing::trace!(?error, status = content, "Context for html parsing error.");
            }
        }
    }
    if !stack.is_empty() {
        tracing::error!("Non-empty stack after handling html content of status.");
        tracing::trace!(
            status = content,
            ?stack,
            "Context for non-empty stack error."
        );
    }
    // Clear trailing whitespace.
    let trimmed_len = result.trim_end().len();
    result.truncate(trimmed_len);
    result
}
