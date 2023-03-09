use syn::{
    spanned::Spanned, Attribute, Error as SynError, Field, Item, ItemFn, ItemStruct,
    Result as SynResult,
};

use crate::naming::{
    CIRCUIT_DEF, CONSTANT_FIELD, PRIVATE_INPUT_FIELD, PUBLIC_INPUT_FIELD, RELATION_OBJECT_DEF,
};

/// Returns the unique field attribute (either `#[constant(..)]`, `#[public_input(..)]` or
/// `#[private_input(..)]`).
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
