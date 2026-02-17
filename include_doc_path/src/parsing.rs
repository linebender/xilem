// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

// ---------------------------------------------------------------------------
// String literal parsing helpers, adapted and modified from the syn crate (v2.0.115).
// Original source: https://docs.rs/syn/2.0.115/src/syn/lit.rs.html
//
// syn is dual-licensed under Apache-2.0 OR MIT.
// Copyright (c) syn contributors.
// ---------------------------------------------------------------------------

use super::compile_error;
use proc_macro::{TokenStream, TokenTree};
use proc_macro2::Span;

fn byte(s: &str, idx: usize) -> u8 {
    s.as_bytes().get(idx).copied().unwrap_or(0)
}

fn next_chr(s: &str) -> char {
    s.chars().next().unwrap_or('\0')
}

fn backslash_x(s: &str) -> Option<(u8, &str)> {
    let b0 = byte(s, 0);
    let b1 = byte(s, 1);
    let hi = char::from(b0).to_digit(16)?;
    let lo = char::from(b1).to_digit(16)?;
    let ch = (hi as u8) * 0x10 + (lo as u8);
    Some((ch, &s[2..]))
}

fn backslash_u(s: &str) -> Option<(char, &str)> {
    if byte(s, 0) != b'{' {
        return None;
    }
    let s = &s[1..];
    let mut ch = 0;
    let mut digits = 0;
    loop {
        let b = byte(s, digits);
        if b == b'}' {
            break;
        }
        if digits == 6 {
            return None;
        }
        let digit = char::from(b).to_digit(16)?;
        ch = ch * 0x10 + digit;
        digits += 1;
    }
    if digits == 0 {
        return None;
    }
    let ch = char::from_u32(ch)?;
    let s = &s[digits + 1..];
    Some((ch, s))
}

/// Parse a cooked (non-raw) string literal, handling all escape sequences.
///
/// `s` must be the full token text including the surrounding double-quotes.
/// Returns `(content, suffix)` on success, where `suffix` is any trailing
/// literal suffix (always empty for valid Rust string literals).
fn parse_lit_str_cooked(mut s: &str) -> Option<(Box<str>, Box<str>)> {
    if byte(s, 0) != b'"' {
        return None;
    }

    s = &s[1..];

    let mut content = String::new();
    'outer: loop {
        let ch = match byte(s, 0) {
            b'"' => break,
            b'\\' => {
                let b = byte(s, 1);
                s = s.get(2..)?;
                match b {
                    b'x' => {
                        let (byte, rest) = backslash_x(s)?;
                        s = rest;
                        if byte > 0x7F {
                            // invalid \x byte in string literal
                            return None;
                        }
                        char::from(byte)
                    }
                    b'u' => {
                        let (ch, rest) = backslash_u(s)?;
                        s = rest;
                        ch
                    }
                    b'n' => '\n',
                    b'r' => '\r',
                    b't' => '\t',
                    b'\\' => '\\',
                    b'0' => '\0',
                    b'\'' => '\'',
                    b'"' => '"',
                    b'\r' | b'\n' => loop {
                        let b = byte(s, 0);
                        match b {
                            b' ' | b'\t' | b'\n' | b'\r' => s = &s[1..],
                            _ => continue 'outer,
                        }
                    },
                    _ => {
                        // unexpected byte after backslash
                        return None;
                    }
                }
            }
            b'\r' => {
                if byte(s, 1) != b'\n' {
                    // bare carriage return not allowed in string
                    return None;
                }
                s = &s[2..];
                '\n'
            }
            _ => {
                let ch = next_chr(s);
                s = s.get(ch.len_utf8()..)?;
                ch
            }
        };
        content.push(ch);
    }

    if !s.starts_with('"') {
        return None;
    }

    let content = content.into_boxed_str();
    let suffix = s[1..].to_owned().into_boxed_str();
    Some((content, suffix))
}

/// Parse a raw string literal (`r"..."`, `r#"..."#`, etc.).
///
/// `s` must be the full token text including the `r`, optional `#` marks, and surrounding quotes.
fn parse_lit_str_raw(mut s: &str) -> Option<(Box<str>, Box<str>)> {
    if byte(s, 0) != b'r' {
        return None;
    }
    s = &s[1..];

    let mut pounds = 0;
    loop {
        match byte(s, pounds) {
            b'#' => pounds += 1,
            b'"' => break,
            _ => return None,
        }
    }
    let close = s.rfind('"').unwrap();
    for end in s.get(close + 1..close + 1 + pounds)?.bytes() {
        if end != b'#' {
            return None;
        }
    }

    let content = s.get(pounds + 1..close)?.to_owned().into_boxed_str();
    let suffix = s[close + 1 + pounds..].to_owned().into_boxed_str();
    Some((content, suffix))
}

// ---------------------------------------------------------------------------
// End of syn-derived code.
// ---------------------------------------------------------------------------

pub(crate) fn parse_string_literal(input: TokenStream) -> Result<(String, Span), TokenStream> {
    let mut iter = input.into_iter();
    // Must have exactly one token
    let token = match iter.next() {
        Some(t) => t,
        None => return Err(compile_error("expected string literal", Span::call_site())),
    };

    if iter.next().is_some() {
        return Err(compile_error(
            "expected only one string literal",
            Span::call_site(),
        ));
    }

    match token {
        TokenTree::Literal(lit) => {
            let span: Span = lit.span().into();
            let raw = lit.to_string();

            // Try raw string literal first (r"..." or r#"..."#)
            if raw.starts_with('r') {
                match parse_lit_str_raw(&raw) {
                    Some((content, _suffix)) => return Ok((String::from(content), span)),
                    None => {
                        return Err(compile_error("invalid raw string literal", span));
                    }
                }
            }

            // Regular (cooked) string literal
            if raw.starts_with('"') {
                match parse_lit_str_cooked(&raw) {
                    Some((content, _suffix)) => return Ok((String::from(content), span)),
                    None => {
                        return Err(compile_error("invalid string literal", span));
                    }
                }
            }

            Err(compile_error(
                "expected a string literal, not a byte string or other literal",
                span,
            ))
        }
        _ => Err(compile_error(
            "expected string literal like \"path/file.png\"",
            Span::call_site(),
        )),
    }
}
