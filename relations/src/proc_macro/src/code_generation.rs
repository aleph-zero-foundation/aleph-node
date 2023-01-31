use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::quote;
use syn::Result as SynResult;

use crate::{
    generation_utils::{
        failing_field_getters, field_backend_decls, field_castings, field_frontend_decls,
        field_rewrites, field_serializations, plain_field_getters, successful_field_getters,
    },
    intermediate_representation::IR,
    naming::{struct_name_with_full, struct_name_with_public, struct_name_without_input},
};

/// Generates the whole code based on the intermediate representation.
pub(super) fn generate_code(ir: IR) -> SynResult<TokenStream2> {
    let imports = &ir.imports;

    let blocks = [
        quote! { #(#imports)* },
        generate_relation_without_input(&ir)?,
        generate_relation_with_public(&ir)?,
        generate_relation_with_full(&ir)?,
        generate_circuit_definitions(&ir),
    ];

    Ok(TokenStream2::from_iter(blocks))
}

/// Generates struct, constructor and getters for the relation object with constants only.
fn generate_relation_without_input(ir: &IR) -> SynResult<TokenStream2> {
    let struct_name = struct_name_without_input(&ir.relation_base_name);
    let const_frontend_decls = field_frontend_decls(&ir.constants);
    let const_backend_decls = field_backend_decls(&ir.constants);
    let const_castings = field_castings(&ir.constants)?;
    let getters = [
        plain_field_getters(&ir.constants),
        failing_field_getters(&ir.public_inputs),
        failing_field_getters(&ir.private_inputs),
    ]
    .concat();

    Ok(quote! {
        pub struct #struct_name {
            #(#const_backend_decls),*
        }
        impl #struct_name {
            pub fn new(#(#const_frontend_decls),*) -> Self {
                Self { #(#const_castings),* }
            }
            #(#getters)*
        }
    })
}

fn generate_public_input_serialization(ir: &IR) -> SynResult<TokenStream2> {
    let accesses = field_serializations(&ir.public_inputs, &Ident::new("self", Span::call_site()));

    Ok(quote! {
        pub fn serialize_public_input(&self) -> ark_std::vec::Vec<ark_bls12_381::Fr> {
            [ #(#accesses),* ].concat()
        }
    })
}

/// Generates struct, constructor, getters, public input serialization and downcasting for the
/// relation object with constants and public input.
fn generate_relation_with_public(ir: &IR) -> SynResult<TokenStream2> {
    let struct_name = struct_name_with_public(&ir.relation_base_name);
    let struct_name_without_input = struct_name_without_input(&ir.relation_base_name);
    let object_ident = Ident::new("obj", Span::call_site());

    let backend_decls = [
        field_backend_decls(&ir.constants),
        field_backend_decls(&ir.public_inputs),
    ]
    .concat();
    let frontend_decls = [
        field_frontend_decls(&ir.constants),
        field_frontend_decls(&ir.public_inputs),
    ]
    .concat();
    let castings = [
        field_castings(&ir.constants)?,
        field_castings(&ir.public_inputs)?,
    ]
    .concat();
    let getters = [
        plain_field_getters(&ir.constants),
        successful_field_getters(&ir.public_inputs),
        failing_field_getters(&ir.private_inputs),
    ]
    .concat();

    let const_rewrites = field_rewrites(&ir.constants, &object_ident);

    let public_input_serialization = generate_public_input_serialization(ir)?;

    Ok(quote! {
        pub struct #struct_name {
            #(#backend_decls),*
        }
        impl #struct_name {
            #[allow(clippy::too_many_arguments)]
            pub fn new(#(#frontend_decls),*) -> Self {
                Self { #(#castings),* }
            }

            #(#getters)*

            #public_input_serialization
        }

        impl From<#struct_name> for #struct_name_without_input {
            fn from(#object_ident: #struct_name) -> Self {
                Self { #(#const_rewrites),* }
            }
        }
    })
}

/// Generates struct, constructor, getters downcasting for the full relation object.
fn generate_relation_with_full(ir: &IR) -> SynResult<TokenStream2> {
    let struct_name = struct_name_with_full(&ir.relation_base_name);
    let struct_name_with_public = struct_name_with_public(&ir.relation_base_name);
    let object_ident = Ident::new("obj", Span::call_site());

    let backend_decls = [
        field_backend_decls(&ir.constants),
        field_backend_decls(&ir.public_inputs),
        field_backend_decls(&ir.private_inputs),
    ]
    .concat();
    let frontend_decls = [
        field_frontend_decls(&ir.constants),
        field_frontend_decls(&ir.public_inputs),
        field_frontend_decls(&ir.private_inputs),
    ]
    .concat();
    let castings = [
        field_castings(&ir.constants)?,
        field_castings(&ir.public_inputs)?,
        field_castings(&ir.private_inputs)?,
    ]
    .concat();

    let getters = [
        plain_field_getters(&ir.constants),
        successful_field_getters(&ir.public_inputs),
        successful_field_getters(&ir.private_inputs),
    ]
    .concat();

    let const_and_public_rewrites = [
        field_rewrites(&ir.constants, &object_ident),
        field_rewrites(&ir.public_inputs, &object_ident),
    ]
    .concat();

    Ok(quote! {
        pub struct #struct_name {
            #(#backend_decls),*
        }
        impl #struct_name {
            #[allow(clippy::too_many_arguments)]
            pub fn new(#(#frontend_decls),*) -> Self {
                Self { #(#castings),* }
            }

            #(#getters)*
        }

        impl From<#struct_name> for #struct_name_with_public {
            fn from(#object_ident: #struct_name) -> Self {
                Self { #(#const_and_public_rewrites),* }
            }
        }
    })
}

/// Generates `ConstraintSynthesizer` implementations.
fn generate_circuit_definitions(ir: &IR) -> TokenStream2 {
    let struct_name_without_input = struct_name_without_input(&ir.relation_base_name);
    let struct_name_with_full = struct_name_with_full(&ir.relation_base_name);

    let body = &ir.circuit_definition.block.stmts;

    quote! {
        impl ark_relations::r1cs::ConstraintSynthesizer<ark_bls12_381::Fr> for #struct_name_without_input {
            fn generate_constraints(
                self,
                cs: ark_relations::r1cs::ConstraintSystemRef<ark_bls12_381::Fr>
            ) -> ark_relations::r1cs::Result<()> {
                if cs.is_in_setup_mode() {
                    #(#body)*
                } else {
                    #[cfg(feature = "std")] {
                        eprintln!("For proof generation, you should use relation object with full input.");
                    }
                    Err(ark_relations::r1cs::SynthesisError::AssignmentMissing)
                }
            }
        }

        impl ark_relations::r1cs::ConstraintSynthesizer<ark_bls12_381::Fr> for #struct_name_with_full {
            fn generate_constraints(
                self,
                cs: ark_relations::r1cs::ConstraintSystemRef<ark_bls12_381::Fr>
            ) -> ark_relations::r1cs::Result<()> {
                    #(#body)*
            }
        }
    }
}
