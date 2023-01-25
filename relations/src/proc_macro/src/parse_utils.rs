use std::collections::{HashMap, HashSet};

use syn::{
    spanned::Spanned, Attribute, Error as SynError, Field, Item, ItemFn, ItemStruct, Lit, Meta,
    MetaList, MetaNameValue, NestedMeta, Result as SynResult,
};

use crate::naming::{
    CIRCUIT_DEF, CONSTANT_FIELD, FIELD_FRONTEND_TYPE, FIELD_PARSER, FIELD_SERIALIZER,
    PRIVATE_INPUT_FIELD, PUBLIC_INPUT_FIELD, RELATION_OBJECT_DEF,
};

/// Returns the unique field attribute (either `#[constant]`, `#[public_input]` or
/// `#[private_input]`).
pub(super) fn get_field_attr(field: &Field) -> SynResult<&Attribute> {
    let attrs = field
        .attrs
        .iter()
        .filter(|a| {
            a.path.is_ident(CONSTANT_FIELD)
                || a.path.is_ident(PUBLIC_INPUT_FIELD)
                || a.path.is_ident(PRIVATE_INPUT_FIELD)
        })
        .collect::<Vec<_>>();
    match &*attrs {
        &[attr] => Ok(attr),
        _ => Err(SynError::new(
            field.span(),
            "Relation field should have exactly one type: constant, public or private input.",
        )),
    }
}

/// Returns mapping from the modifier name to the literal value. Expects only common modifiers in
/// `attr`.
pub(super) fn get_relation_field_config(attr: &Attribute) -> SynResult<HashMap<String, String>> {
    let permissible_config = HashSet::from([FIELD_FRONTEND_TYPE, FIELD_PARSER]);
    get_field_config(attr, &permissible_config)
}

/// Returns mapping from the modifier name to the literal value. Accepts all modifiers.
pub(super) fn get_public_input_field_config(
    attr: &Attribute,
) -> SynResult<HashMap<String, String>> {
    let permissible_config = HashSet::from([FIELD_FRONTEND_TYPE, FIELD_PARSER, FIELD_SERIALIZER]);
    get_field_config(attr, &permissible_config)
}

/// Returns mapping from the modifier name to the literal value.
fn get_field_config(
    attr: &Attribute,
    permissible_config: &HashSet<&str>,
) -> SynResult<HashMap<String, String>> {
    let err = SynError::new(attr.span(), "Invalid attribute syntax");

    match attr.parse_meta()? {
        Meta::Path(_) => Ok(HashMap::new()),
        Meta::NameValue(_) => Err(err),
        Meta::List(MetaList { nested, .. }) => {
            let mut config = HashMap::new();
            for nm in nested {
                match nm {
                    NestedMeta::Meta(Meta::NameValue(MetaNameValue { path, lit, .. })) => {
                        let path = path.get_ident().map(|i| i.to_string()).ok_or(err.clone())?;
                        if !permissible_config.contains(path.as_str()) {
                            return Err(err);
                        }
                        let lit = match lit {
                            Lit::Str(lit_str) => lit_str.value(),
                            Lit::Int(lit_int) => lit_int.token().to_string(),
                            _ => return Err(err),
                        };
                        config.insert(path, lit);
                    }
                    _ => return Err(err),
                }
            }

            Ok(config)
        }
    }
}

/// Tries casting `item` to `ItemStruct` only when it is attributed with
/// `#[relation_object_definition]`.
pub(super) fn as_relation_object_def(item: &Item) -> Option<ItemStruct> {
    match item {
        Item::Struct(item_struct) => item_struct
            .attrs
            .iter()
            .any(|a| a.path.is_ident(RELATION_OBJECT_DEF))
            .then_some(item_struct.clone()),
        _ => None,
    }
}

/// Tries casting `item` to `ItemFn` only when it is attributed with `#[circuit_definition]`.
pub(super) fn as_circuit_def(item: &Item) -> Option<ItemFn> {
    match item {
        Item::Fn(item_fn) => item_fn
            .attrs
            .iter()
            .any(|a| a.path.is_ident(CIRCUIT_DEF))
            .then_some(item_fn.clone()),
        _ => None,
    }
}
