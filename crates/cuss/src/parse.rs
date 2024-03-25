// Copyright 2023 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Parser for stylesheets.

use crate::{
    rules::{Declaration, Rule, Stylesheet, Value},
    selector::{Combinator, ComplexSelector, CompoundSelector, TypeSelector},
    symbol::SymbolPool,
    Symbol,
};

// This should be expanded to at least point to an offset in the source.
#[derive(Debug)]
pub struct ParseError(&'static str);

pub struct Parser<'a> {
    pool: &'a mut SymbolPool,
    input: &'a str,
    ix: usize,
}

// More than a little tricky, but I didn't want to call heavy string methods.
fn count_utf8(b: u8) -> usize {
    ((0x4322000011111111u64 >> ((b >> 2) & 0x3c)) & 15) as usize
}

impl<'a> Parser<'a> {
    pub fn new(input: &'a str, pool: &'a mut SymbolPool) -> Parser<'a> {
        let ix = 0;
        Parser { pool, input, ix }
    }

    // This will be called at the start of most tokens.
    fn consume_comments(&mut self) -> Result<(), ParseError> {
        while self.input[self.ix..].starts_with("/*") {
            if let Some(i) = self.input[self.ix + 2..].find("*/") {
                self.ix += i + 4;
            } else {
                return Err(ParseError("unclosed comment"));
            }
        }
        Ok(())
    }

    // Discussion question: do we ever need the int/float distinction downstream?
    fn number(&mut self) -> Option<f64> {
        self.consume_comments().ok()?;
        let tail = &self.input[self.ix..];
        let mut i = 0;
        let mut valid = false;
        if matches!(tail.as_bytes().first(), Some(b'+') | Some(b'-')) {
            i += 1;
        }
        while let Some(c) = tail.as_bytes().get(i) {
            if c.is_ascii_digit() {
                valid = true;
                i += 1;
            } else {
                break;
            }
        }
        if let Some(b'.') = tail.as_bytes().get(i) {
            if let Some(c) = tail.as_bytes().get(i + 1) {
                if c.is_ascii_digit() {
                    valid = true;
                    i += 2;
                    while let Some(c) = tail.as_bytes().get(i) {
                        if c.is_ascii_digit() {
                            i += 1;
                        } else {
                            break;
                        }
                    }
                }
            }
        }
        if matches!(tail.as_bytes().get(i), Some(b'e') | Some(b'E')) {
            let mut j = i + 1;
            if matches!(tail.as_bytes().get(j), Some(b'+') | Some(b'-')) {
                j += 1;
            }
            if let Some(c) = tail.as_bytes().get(j) {
                if c.is_ascii_digit() {
                    i = j + 1;
                    while let Some(c) = tail.as_bytes().get(i) {
                        if c.is_ascii_digit() {
                            i += 1;
                        } else {
                            break;
                        }
                    }
                }
            }
        }
        if valid {
            // For this parse to fail would be strange, but we'll be careful.
            if let Ok(value) = tail[..i].parse() {
                self.ix += i;
                return Some(value);
            }
        }
        None
    }

    // Complies with ident-token production with three exceptions:
    // Escapes are not supported.
    // Non-ASCII characters are not supported.
    // Result is case sensitive.
    fn ident(&mut self) -> Option<Symbol> {
        self.ident_inner(0)
    }

    fn ident_inner(&mut self, i_init: usize) -> Option<Symbol> {
        // This does *not* strip initial whitespace.
        let tail = &self.input[self.ix..];
        let mut i = i_init;
        while i < tail.len() {
            let b = tail.as_bytes()[i];
            if b.is_ascii_alphabetic()
                || b == b'_'
                || b == b'-'
                || ((i >= 2 || i == 1 && tail.as_bytes()[i_init] != b'-')
                    && b.is_ascii_digit())
            {
                i += 1;
            } else {
                break;
            }
        }
        // Reject '', '-', and anything starting with '--'
        let mut j = i_init;
        while j < i.min(i_init + 2) {
            if tail.as_bytes()[j] == b'-' {
                j += 1;
            } else {
                self.ix += i;
                return Some(self.pool.intern(&tail[..i]));
            }
        }
        None
    }

    /// Scan a pseudoclass (symbol prefixed with colon).
    ///
    /// The returned symbol contains the colon.
    fn pseudoclass(&mut self) -> Option<Symbol> {
        if self.input[self.ix..].starts_with(':') {
            self.ident_inner(1)
        } else {
            None
        }
    }

    // Assumes the starting code point has already been consumed
    fn string(&mut self, ending: u8) -> Result<String, ParseError> {
        let mut result = String::new();
        let tail = &self.input[self.ix..];
        let mut i = 0;
        while i < tail.len() {
            let b = tail.as_bytes()[i];
            if b == ending {
                self.ix += i + 1;
                return Ok(result);
            } else if b == b'\n' {
                return Err(ParseError("unclosed string at end of line"));
            } else if b == b'\\' {
                if let Some(b'\n') = tail.as_bytes().get(i + 1) {
                    i += 2;
                } else {
                    todo!("escapes nyi");
                }
            } else {
                let len = count_utf8(b);
                result.push_str(&tail[i..][..len]);
                i += count_utf8(b);
            }
        }
        Err(ParseError("unclosed string at end of stylesheet"))
    }

    fn ch(&mut self, ch: u8) -> bool {
        if self.consume_comments().is_err() {
            return false;
        }
        self.raw_ch(ch)
    }

    fn raw_ch(&mut self, ch: u8) -> bool {
        if self.input[self.ix..].as_bytes().first() == Some(&ch) {
            self.ix += 1;
            true
        } else {
            false
        }
    }

    fn ws_one(&mut self) -> bool {
        if self.consume_comments().is_err() {
            return false;
        }
        let tail = &self.input[self.ix..];
        let mut i = 0;
        while let Some(&b) = tail.as_bytes().get(i) {
            if !(b == b' ' || b == b'\t' || b == b'\r' || b == b'\n') {
                break;
            }
            i += 1;
        }
        self.ix += i;
        i > 0
    }

    fn ws(&mut self) -> bool {
        if !self.ws_one() {
            return false;
        }
        while self.consume_comments().is_ok() {
            if !self.ws_one() {
                break;
            }
        }
        true
    }

    fn at_eof(&self) -> bool {
        self.ix == self.input.len()
    }

    fn color(&mut self) -> Result<u32, ParseError> {
        let tail = &self.input[self.ix..];
        let mut i = 0;
        while i < tail.len() {
            let b = tail.as_bytes()[i];
            if !b.is_ascii_hexdigit() {
                break;
            }
            i += 1;
        }
        if !(i == 3 || i == 6) {
            return Err(ParseError("color must be 3 or 6 hex digits"));
        }
        let raw = u32::from_str_radix(&tail[..i], 16).unwrap();
        self.ix += i;
        if i == 3 {
            Ok((((raw & 0xf00) << 8) | (raw & 0xf0) << 4 | (raw & 0xf)) * 0x11)
        } else {
            Ok(raw)
        }
    }

    fn compound_selector(&mut self) -> Result<Option<CompoundSelector>, ParseError> {
        let mut type_selector = None;
        if self.ch(b'*') {
            type_selector = Some(TypeSelector::Wild);
        } else if let Some(tag) = self.ident() {
            type_selector = Some(TypeSelector::Ident(tag));
        }
        let mut id_selector = None;
        let mut class_selectors = Vec::new();
        loop {
            if self.ch(b'#') {
                let id_token = self.ident().ok_or(ParseError("missing id"))?;
                // TODO: setting id multiply increases specificity
                id_selector = Some(id_token);
            } else if let Some(pseudoclass) = self.pseudoclass() {
                class_selectors.push(pseudoclass);
            } else if self.ch(b'.') {
                self.consume_comments()?;
                let class_token = self.ident().ok_or(ParseError("missing class"))?;
                class_selectors.push(class_token);
            } else {
                break;
            }
        }
        if type_selector.is_none() && id_selector.is_none() && class_selectors.is_empty() {
            Ok(None)
        } else {
            class_selectors.sort();
            // TODO: remove duplicates, but count specificity
            Ok(Some(CompoundSelector {
                type_selector,
                id_selector,
                class_selectors,
            }))
        }
    }

    pub fn complex_selector(&mut self) -> Result<Option<ComplexSelector>, ParseError> {
        if let Some(first) = self.compound_selector()? {
            let mut tail = Vec::new();
            loop {
                let ws = self.ws();
                let child = self.ch(b'>');
                if child {
                    self.ws();
                }
                if !(ws || child) {
                    break;
                }
                let combinator = if child {
                    Combinator::Child
                } else {
                    Combinator::Descendant
                };
                if let Some(next) = self.compound_selector()? {
                    tail.push((combinator, next));
                } else {
                    if child {
                        return Err(ParseError("missing child"));
                    }
                    break;
                }
            }
            Ok(Some(ComplexSelector { first, tail }))
        } else {
            Ok(None)
        }
    }

    pub fn selector_list(&mut self) -> Result<Option<Vec<ComplexSelector>>, ParseError> {
        if let Some(rule) = self.complex_selector()? {
            let mut result = vec![rule];
            self.ws();
            while self.ch(b',') {
                self.ws();
                if let Some(rule) = self.complex_selector()? {
                    result.push(rule);
                } else {
                    return Err(ParseError("expected rule after comma"));
                }
                self.ws();
            }
            Ok(Some(result))
        } else {
            Ok(None)
        }
    }

    pub fn value(&mut self) -> Result<Option<Value>, ParseError> {
        if self.ch(b'"') {
            Ok(Some(Value::String(self.string(b'"')?)))
        } else if self.ch(b'\'') {
            Ok(Some(Value::String(self.string(b'\'')?)))
        } else if let Some(number) = self.number() {
            if self.raw_ch(b'%') {
                Ok(Some(Value::Percent(number)))
            } else if let Some(unit) = self.ident() {
                Ok(Some(Value::Dimension(number, unit)))
            } else {
                Ok(Some(Value::Number(number)))
            }
        } else if self.ch(b'#') {
            Ok(Some(Value::Color(self.color()?)))
        } else if let Some(ident) = self.ident() {
            if self.raw_ch(b'(') {
                let mut args = vec![];
                self.ws();
                if let Some(value) = self.value()? {
                    args.push(value);
                    self.ws();
                }
                while self.ch(b',') {
                    self.ws();
                    if let Some(value) = self.value()? {
                        args.push(value);
                        self.ws();
                    } else {
                        return Err(ParseError("expected value in function arg"));
                    }
                }
                if !self.ch(b')') {
                    return Err(ParseError("expected close paren"));
                }
                return Ok(Some(Value::Function(ident, args)));
            }
            Ok(Some(Value::Symbol(ident)))
        } else {
            Ok(None)
        }
    }

    pub fn declaration(&mut self) -> Result<Option<Declaration>, ParseError> {
        self.consume_comments()?;
        if let Some(name) = self.ident() {
            self.ws();
            if !self.ch(b':') {
                return Err(ParseError("expected colon after name"));
            }
            self.ws();
            let mut values = vec![];
            while let Some(value) = self.value()? {
                values.push(value);
                self.ws();
            }
            if values.is_empty() {
                Err(ParseError("expected value in declaration"))
            } else {
                Ok(Some(Declaration { name, values }))
            }
        } else {
            Ok(None)
        }
    }

    pub fn style_rule(&mut self) -> Result<Option<Rule>, ParseError> {
        if let Some(selectors) = self.selector_list()? {
            if !self.ch(b'{') {
                return Err(ParseError("expected block after selectors"));
            }
            let mut decls = vec![];
            loop {
                self.ws();
                if self.ch(b'}') {
                    break;
                }
                if self.ch(b';') {
                    continue;
                }
                if let Some(decl) = self.declaration()? {
                    decls.push(decl);
                } else {
                    return Err(ParseError("expected declaration"));
                }
            }
            let rule = Rule { selectors, decls };
            Ok(Some(rule))
        } else {
            Ok(None)
        }
    }

    pub fn stylesheet(&mut self) -> Result<Stylesheet, ParseError> {
        let mut rules = vec![];
        self.ws();
        while let Some(rule) = self.style_rule()? {
            rules.push(rule);
            self.ws();
        }
        if !self.at_eof() {
            return Err(ParseError("extra unparsed stuff before eof"));
        }
        Ok(Stylesheet(rules))
    }
}
