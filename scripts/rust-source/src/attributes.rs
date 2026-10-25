//! Attribute contracts retain unknown predicates for the platform caller.
use serde_json::{json, Value};
use syn::parse::Parser;
use syn::{punctuated::Punctuated, Attribute, Item, Meta, Token};

pub fn predicate(meta: &Meta) -> Value {
    match meta {
        Meta::Path(path) => json!({"name": path.get_ident().map(ToString::to_string)}),
        Meta::NameValue(value) => key_value(value),
        Meta::List(list) => {
            let args = Punctuated::<Meta, Token![,]>::parse_terminated.parse2(list.tokens.clone());
            json!({"name": list.path.get_ident().map(ToString::to_string),
                   "args": args.ok().map(|args| args.iter().map(predicate).collect::<Vec<_>>())})
        }
    }
}

fn key_value(meta: &syn::MetaNameValue) -> Value {
    let value = match &meta.value {
        syn::Expr::Lit(expr) => match &expr.lit {
            syn::Lit::Str(text) => Some(text.value()),
            _ => None,
        },
        _ => None,
    };
    json!({"name": meta.path.get_ident().map(ToString::to_string), "value": value})
}

pub fn guards(attrs: &[Attribute]) -> Vec<Value> {
    attrs
        .iter()
        .filter(|attr| attr.path().is_ident("cfg"))
        .map(|attr| {
            attr.parse_args::<Meta>()
                .map(|meta| predicate(&meta))
                .unwrap_or(Value::Null)
        })
        .collect()
}

fn test_guard(guard: &Value) -> bool {
    if guard["name"] == "test" {
        return true;
    }
    guard["name"] == "all" && guard["args"][0]["name"] == "test"
}

pub fn test_only(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        let name: Vec<_> = attr
            .path()
            .segments
            .iter()
            .map(|part| part.ident.to_string())
            .collect();
        name == ["test"] || name == ["tokio", "test"]
    }) || guards(attrs).iter().any(test_guard)
}

pub fn path(attrs: &[Attribute]) -> Option<String> {
    attrs.iter().find_map(|attr| {
        if !attr.path().is_ident("path") {
            return None;
        }
        key_value(attr.meta.require_name_value().ok()?)["value"]
            .as_str()
            .map(str::to_owned)
    })
}

pub fn item(item: &Item) -> &[Attribute] {
    // Executable items and containers can enclose further authored owners.
    match item {
        Item::Const(value) => &value.attrs,
        Item::Fn(value) => &value.attrs,
        Item::ForeignMod(value) => &value.attrs,
        Item::Impl(value) => &value.attrs,
        Item::Mod(value) => &value.attrs,
        Item::Static(value) => &value.attrs,
        Item::Trait(value) => &value.attrs,
        _ => declaration_attributes(item),
    }
}

fn declaration_attributes(item: &Item) -> &[Attribute] {
    match item {
        Item::Enum(value) => &value.attrs,
        Item::Struct(value) => &value.attrs,
        Item::TraitAlias(value) => &value.attrs,
        Item::Type(value) => &value.attrs,
        Item::Union(value) => &value.attrs,
        _ => external_attributes(item),
    }
}

fn external_attributes(item: &Item) -> &[Attribute] {
    match item {
        Item::ExternCrate(value) => &value.attrs,
        Item::Macro(value) => &value.attrs,
        Item::Use(value) => &value.attrs,
        _ => &[],
    }
}
