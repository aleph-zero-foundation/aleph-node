use std::fmt::Display;

use proc_macro2::Ident;
use quote::format_ident;

pub(super) const RELATION_OBJECT_DEF: &str = "relation_object_definition";
pub(super) const CIRCUIT_DEF: &str = "circuit_definition";

pub(super) const CONSTANT_FIELD: &str = "constant";
pub(super) const PUBLIC_INPUT_FIELD: &str = "public_input";
pub(super) const PRIVATE_INPUT_FIELD: &str = "private_input";

pub(super) fn struct_name_without_input<T: Display>(relation_base_name: T) -> Ident {
    format_ident!("{relation_base_name}WithoutInput")
}

pub(super) fn struct_name_with_public<T: Display>(relation_base_name: T) -> Ident {
    format_ident!("{relation_base_name}WithPublicInput")
}

pub(super) fn struct_name_with_full<T: Display>(relation_base_name: T) -> Ident {
    format_ident!("{relation_base_name}WithFullInput")
}
