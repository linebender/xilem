// Copyright 2023 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Simple test program.

// Note: this should probably be moved to examples.

use cuss::{Parser, MatchState, Matcher, ResolveState, Resolver, Symbol, SymbolPool};

fn test_resolve() {
    let mut pool = SymbolPool::new();
    let inp = "hr {
        margin: 20px 0;
        border: 0;
        border-top: 1px dashed #c5c5c5;
        border-bottom: 1px dashed #f7f7f7;
    }
    .learn a {
        font-weight: normal;
        text-decoration: none;
        color: #b83f45;
    }
    .todo-list li:hover .destroy {
        display: block;
    }
    .learn a:hover {
        text-decoration: underline;
        color: #787e7e;
    }
    
    .learn h3,
    .learn h4,
    .learn h5 {
        margin: 10px 0;
        font-weight: 500;
        line-height: 1.2;
        color: #000;
    }
    body {
        font-family: Inconsolata;
    }
    ";
    let mut l = Parser::new(inp, &mut pool);
    let ss = l.stylesheet().unwrap();
    println!("{ss:?}");
    let mut r = Resolver::new(ss);
    let state = ResolveState::default();
    let tip = r.step_id(state, None);
    let tip = r.step_tag(tip, Symbol::BODY);
    let tip = r.step_class(tip, pool.intern("learn"));
    let tip = r.step_class_end(tip);
    let state = r.resolve(tip);
    println!("{:?}", r.props(state));
    let tip = r.step_id(state, None);
    let tip = r.step_tag(tip, Symbol::A);
    let tip = r.step_class(tip, pool.intern(":hover"));
    let tip = r.step_class_end(tip);
    let state = r.resolve(tip);
    println!("{:?}", r.props(state));
}

#[allow(unused)]
fn test_value() {
    let arg = std::env::args().nth(1).unwrap();
    let mut pool = SymbolPool::new();
    let mut l = Parser::new(&arg, &mut pool);
    let n = l.value();
    println!("{n:?}")
}

#[allow(unused)]
fn test_match() {
    let mut pool = SymbolPool::new();
    let inp = "body div#id.class > .child > * > leaf";
    let mut l = Parser::new(inp, &mut pool);
    let sels = vec![l.complex_selector().unwrap().unwrap()];
    let mut m = Matcher::new(sels);
    let state = MatchState::default();
    let tip = m.step_id(state, None);
    let tip = m.step_tag(tip, pool.intern("body"));
    let tip = m.step_class_end(tip);
    let state = m.merge(state, tip);
    let tip = m.step_id(state, Some(pool.intern("id")));
    let tip = m.step_tag(tip, pool.intern("div"));
    let tip = m.step_class(tip, pool.intern("class"));
    let tip = m.step_class_end(tip);
    let state = m.merge(state, tip);
    let tip = m.step_id(state, None);
    let tip = m.step_tag(tip, pool.intern("div"));
    let tip = m.step_class(tip, pool.intern("child"));
    let tip = m.step_class_end(tip);
    let state = m.merge(state, tip);
    let tip = m.step_id(state, None);
    let tip = m.step_tag(tip, pool.intern("div"));
    let tip = m.step_class_end(tip);
    let state = m.merge(state, tip);
    let tip = m.step_id(state, None);
    let tip = m.step_tag(tip, pool.intern("leaf"));
    let tip = m.step_class_end(tip);
    m.dump_state(tip);
    let state = m.merge(state, tip);
    //let tip = m.step_id(state, None);
    //let tip = m.step_tag(tip, pool.intern("leaf"));
    //let tip = m.step_class_end(tip);
    //let state = m.merge(state, tip);
    m.dump_state(state);
}

fn main() {
    test_resolve();
}
