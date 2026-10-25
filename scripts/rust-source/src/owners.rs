//! Authored ownership and byte spans, independent of native coverage counters.
use proc_macro2::Span;
use serde_json::{json, Value};
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{Attribute, Block, Signature, Visibility};

use crate::{attributes, score};

#[derive(Default)]
pub struct Owners {
    pub records: Vec<Value>,
    offset: usize,
    guards: Vec<Value>,
    test_only: bool,
}

impl Owners {
    pub fn new(offset: usize) -> Self {
        Self {
            offset,
            ..Self::default()
        }
    }

    fn enter(&mut self, attrs: &[Attribute]) -> (usize, bool) {
        let previous = (self.guards.len(), self.test_only);
        self.guards.extend(attributes::guards(attrs));
        self.test_only |= attributes::test_only(attrs);
        previous
    }

    fn leave(&mut self, previous: (usize, bool)) {
        self.guards.truncate(previous.0);
        self.test_only = previous.1;
    }

    fn record(&mut self, kind: &str, name: String, span: Span, extra: Value) {
        self.records.push(json!({"kind": kind, "name": name,
            "first": span.start().line, "last": span.end().line,
            "start": self.offset + span.byte_range().start, "end": self.offset + span.byte_range().end,
            "guards": self.guards, "test_only": self.test_only, "detail": extra}));
    }

    fn function(&mut self, sig: &Signature, body: &Block, visibility: &Visibility) {
        let start = match visibility {
            Visibility::Inherited => sig.span(),
            _ => visibility.span(),
        };
        let span = start.join(body.span()).expect("one source file");
        self.record(
            "function",
            sig.ident.to_string(),
            span,
            json!({"score": score::function(sig, body)}),
        );
    }
}

impl<'ast> Visit<'ast> for Owners {
    fn visit_item(&mut self, node: &'ast syn::Item) {
        let previous = self.enter(attributes::item(node));
        visit::visit_item(self, node);
        self.leave(previous);
    }

    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        self.function(&node.sig, &node.block, &node.vis);
        visit::visit_item_fn(self, node);
    }

    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        let previous = self.enter(&node.attrs);
        self.function(&node.sig, &node.block, &node.vis);
        visit::visit_impl_item_fn(self, node);
        self.leave(previous);
    }

    fn visit_trait_item_fn(&mut self, node: &'ast syn::TraitItemFn) {
        let previous = self.enter(&node.attrs);
        if let Some(body) = &node.default {
            self.function(&node.sig, body, &Visibility::Inherited);
        }
        visit::visit_trait_item_fn(self, node);
        self.leave(previous);
    }

    fn visit_item_mod(&mut self, node: &'ast syn::ItemMod) {
        self.record(
            "module",
            node.ident.to_string(),
            node.span(),
            json!({"inline": node.content.is_some(), "path": attributes::path(&node.attrs)}),
        );
        visit::visit_item_mod(self, node);
    }

    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        if node.path.is_ident("include") {
            if let Ok(path) = syn::parse2::<syn::LitStr>(node.tokens.clone()) {
                self.record("include", path.value(), node.span(), Value::Null);
            }
        }
        // Macro token trees stay opaque: no source rewriting or expansion.
    }
}
