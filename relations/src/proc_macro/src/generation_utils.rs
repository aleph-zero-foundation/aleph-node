use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{spanned::Spanned, Error as SynError, Result as SynResult};

use crate::intermediate_representation::{PublicInputField, RelationField};

/// Forcibly extracts ident from the field.
fn get_ident(f: &RelationField) -> &Ident {
    f.field
        .ident
        .as_ref()
        .expect("Expected struct with named fields")
}

/// Applies `mapper` to every element in `fields` with its ident extracted.
fn map_fields_with_ident<T, F: Into<RelationField> + Clone, M: Fn(&RelationField, &Ident) -> T>(
    fields: &[F],
    mapper: M,
) -> Vec<T> {
    fields
        .iter()
        .map(|f| {
            let f = f.clone().into();
            mapper(&f, get_ident(&f))
        })
        .collect()
}

/// Translates every element in `fields` to either:
///  -  `<ident>: <backend_type>`, if `frontend_type` wasn't specified, or
///  -  `<ident>: <frontend_type>` otherwise.
pub(super) fn field_frontend_decls<F: Into<RelationField> + Clone>(
    fields: &[F],
) -> Vec<TokenStream2> {
    map_fields_with_ident(fields, |rf, ident| {
        let maybe_frontend_type = &rf.frontend_type;
        let backend_type = &rf.field.ty;

        maybe_frontend_type.as_ref().map_or_else(
            || quote! { #ident: #backend_type },
            |ft| {
                let ft = Ident::new(ft.as_str(), Span::call_site());
                quote! { #ident: #ft }
            },
        )
    })
}

/// Translates every element in `fields` to `<ident>: <backend_type>`.
pub(super) fn field_backend_decls<F: Into<RelationField> + Clone>(
    fields: &[F],
) -> Vec<TokenStream2> {
    map_fields_with_ident(fields, |rf, ident| {
        let ty = &rf.field.ty;
        quote! { #ident: #ty }
    })
}

/// Translates every element in `fields` to either:
///  -  `vec![ <obj> . <ident> ]` if `serialize_with` wasn't specified, or
///  -  `<serializer>( & <obj> . <ident> )` otherwise.
pub(super) fn field_serializations(fields: &[PublicInputField], obj: &Ident) -> Vec<TokenStream2> {
    fields
        .iter()
        .map(|f| {
            let ident = get_ident(&f.inner);
            match &f.serialize_with {
                None => quote! { ark_std::vec![ #obj . #ident ] },
                Some(serializer) => {
                    let serializer = Ident::new(serializer, Span::call_site());
                    quote! { #serializer ( & #obj . #ident ) }
                }
            }
        })
        .collect()
}

/// Translates every element in `fields` to `<ident>: <obj> . <ident>`.
pub(super) fn field_rewrites<F: Into<RelationField> + Clone>(
    fields: &[F],
    obj: &Ident,
) -> Vec<TokenStream2> {
    map_fields_with_ident(fields, |_, ident| {
        quote! { #ident : #obj . #ident }
    })
}

/// Translates every element in `fields` to either:
///  -  `<ident>` if neither `frontend_type` nor `parse_with` was specified, or
///  -  `<ident> : <ident> . into()` if `frontend_type` was specified, but `parse_with` wasn't, or
///  -  `<ident> : <parser> ( <ident> )` if  both `frontend_type` and `parse_with` were specified.
/// Otherwise, method fails.
pub(super) fn field_castings<F: Into<RelationField> + Clone>(
    fields: &[F],
) -> SynResult<Vec<TokenStream2>> {
    map_fields_with_ident(fields, |rf, ident| {
        let maybe_frontend_type = &rf.frontend_type;
        let maybe_parser = &rf.parse_with;

        match (maybe_frontend_type, maybe_parser) {
            (None, None) => Ok(quote! { #ident }),
            (None, Some(_)) => Err(SynError::new(
                rf.field.span(),
                "Parser is provided, but frontend type is absent.",
            )),
            (Some(_), None) => Ok(quote! { #ident : #ident . into() }),
            (Some(_), Some(parser)) => {
                let parser = Ident::new(parser, Span::call_site());
                Ok(quote! { #ident : #parser ( #ident ) })
            }
        }
    })
    .into_iter()
    .collect()
}

/// Translates every element in `fields` to `<self> . <ident>`.
pub(super) fn plain_field_getters<F: Into<RelationField> + Clone>(
    fields: &[F],
) -> Vec<TokenStream2> {
    map_fields_with_ident(fields, |rf, ident| {
        let backend_type = &rf.field.ty;
        quote! {
            pub fn #ident(&self) -> & #backend_type {
                &self . #ident
            }
        }
    })
}

/// Translates every element in `fields` to:
/// ```ignore
/// pub fn <ident>(&self) -> Result<<backend_type>> {
///     Ok(&self . <ident>)
/// }
/// ```
pub(super) fn successful_field_getters<F: Into<RelationField> + Clone>(
    fields: &[F],
) -> Vec<TokenStream2> {
    map_fields_with_ident(fields, |rf, ident| {
        let backend_type = &rf.field.ty;
        quote! {
            pub fn #ident(&self) -> core::result::Result<& #backend_type, ark_relations::r1cs::SynthesisError> {
                Ok(&self . #ident)
            }
        }
    })
}

/// Translates every element in `fields` to:
/// ```ignore
/// pub fn <ident>(&self) -> Result<<backend_type>> {
///     Err(SynthesisError::AssignmentMissing)
/// }
/// ```
pub(super) fn failing_field_getters<F: Into<RelationField> + Clone>(
    fields: &[F],
) -> Vec<TokenStream2> {
    map_fields_with_ident(fields, |rf, ident| {
        let backend_type = &rf.field.ty;
        quote! {
            pub fn #ident(&self) -> core::result::Result<& #backend_type, ark_relations::r1cs::SynthesisError> {
                Err(ark_relations::r1cs::SynthesisError::AssignmentMissing)
            }
        }
    })
}
