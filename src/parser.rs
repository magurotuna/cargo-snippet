use syn::{parse_file, Attribute, File, Item, MetaItem, NestedMetaItem};
use quote::ToTokens;

macro_rules! get_attrs_impl {
    ($arg: expr, $($v: path), *) => {
        {
            match $arg {
                $(
                    &$v(ref x) => Some(&x.attrs),
                )*
                _ => None
            }
        }
    }
}

fn get_attrs(item: &Item) -> Option<&Vec<Attribute>> {
    // All Item variants except Item::Verbatim
    get_attrs_impl!(
        item,
        Item::ExternCrate,
        Item::Use,
        Item::Static,
        Item::Const,
        Item::Fn,
        Item::Mod,
        Item::ForeignMod,
        Item::Type,
        Item::Struct,
        Item::Enum,
        Item::Union,
        Item::Trait,
        Item::Impl,
        Item::Macro,
        Item::Macro2
    )
}

macro_rules! remove_snippet_attr_impl {
    ($arg: expr, $($v: path), *) => {
        {
            match $arg {
                $(
                    &mut $v(ref mut x) => {
                        x.attrs.retain(|attr| {
                            attr.meta_item().map(|m| m.name()!="snippet").unwrap_or(true)
                        });
                    },
                )*
                _ => ()
            }
        }
    }
}

fn remove_snippet_attr(item: &mut Item) {
    remove_snippet_attr_impl!(
        item,
        Item::ExternCrate,
        Item::Use,
        Item::Static,
        Item::Const,
        Item::Fn,
        Item::Mod,
        Item::ForeignMod,
        Item::Type,
        Item::Struct,
        Item::Enum,
        Item::Union,
        Item::Trait,
        Item::Impl,
        Item::Macro,
        Item::Macro2
    )
}

fn unquote(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();

    if chars.len() >= 2 && chars.first() == Some(&'"') && chars.last() == Some(&'"') {
        chars[1..chars.len() - 1].iter().collect()
    } else {
        chars.iter().collect()
    }
}

fn get_snippet_name(attr: &Attribute) -> Option<String> {
    attr.meta_item().and_then(|metaitem| {
        if metaitem.name() != "snippet" {
            return None;
        }

        match metaitem {
            // #[snippet(name="..")]
            MetaItem::List(list) => list.nested
                .iter()
                .filter_map(|item| {
                    if let &NestedMetaItem::MetaItem(MetaItem::NameValue(ref nv)) = item {
                        if format!("{}", nv.ident) == "name" {
                            Some(unquote(&format!("{:?}", nv.lit)))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .next(),
            // #[snippet=".."]
            MetaItem::NameValue(nv) => Some(unquote(&format!("{}", nv.lit.into_tokens()))),
            _ => None,
        }
    })
}

// snippet name and snippet code (not formatted)
fn get_snippet_from_item(mut item: Item) -> Option<(String, String)> {
    let snip_name = get_attrs(&item).and_then(|attrs| {
        attrs
            .iter()
            .filter_map(|attr| get_snippet_name(attr))
            .next()
    });

    snip_name.map(|name| {
        remove_snippet_attr(&mut item);
        (name, format!("{}", item.into_tokens()))
    })
}

fn get_snippet_from_file(file: File) -> Vec<(String, String)> {
    let mut res = Vec::new();

    // whole code is snippet
    let snip_name = file.attrs
        .iter()
        .filter_map(|attr| get_snippet_name(attr))
        .next();

    if let Some(name) = snip_name {
        let mut file = file.clone();
        file.attrs.retain(|attr| {
            attr.meta_item()
                .map(|m| m.name() != "snippet")
                .unwrap_or(true)
        });
        file.items.iter_mut().for_each(|item| {
            remove_snippet_attr(item);
        });
        res.push((name, format!("{}", file.into_tokens())));
    }

    res.extend(
        file.items
            .into_iter()
            .filter_map(|item| get_snippet_from_item(item)),
    );

    res
}

pub fn parse_snippet(src: &str) -> Vec<(String, String)> {
    parse_file(src)
        .ok()
        .map(|file| get_snippet_from_file(file))
        .unwrap_or(Vec::new())
}