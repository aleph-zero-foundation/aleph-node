use proc_macro2::{Ident, Span};
use syn::{
    spanned::Spanned, Error as SynError, Field, Fields, FieldsNamed, Item, ItemFn, ItemMod,
    ItemStruct, ItemUse, Result as SynResult, Visibility,
};

use crate::{
    naming::{
        CONSTANT_FIELD, FIELD_FRONTEND_TYPE, FIELD_PARSER, FIELD_SERIALIZER, PRIVATE_INPUT_FIELD,
        PUBLIC_INPUT_FIELD,
    },
    parse_utils::{
        as_circuit_def, as_relation_object_def, get_field_attr, get_public_input_field_config,
        get_relation_field_config,
    },
};

/// Intermediate representation of the source code.
pub(super) struct IR {
    /// Prefix for the new structs.
    pub relation_base_name: Ident,

    /// All constants fields with modifiers.
    pub constants: Vec<RelationField>,
    /// All public input fields with modifiers.
    pub public_inputs: Vec<PublicInputField>,
    /// All private input fields with modifiers.
    pub private_inputs: Vec<RelationField>,

    /// Circuit definition method.
    pub circuit_definition: ItemFn,

    /// Imports to be inherited.
    pub imports: Vec<ItemUse>,
}

/// Common data for constant, public and private inputs.
#[derive(Clone)]
pub(super) struct RelationField {
    /// The source item AST.
    pub field: Field,
    /// The value of the `frontend_type` modifier, if any.
    pub frontend_type: Option<String>,
    /// The value of the `parse_with` modifier, if any.
    pub parse_with: Option<String>,
}

impl TryFrom<Field> for RelationField {
    type Error = SynError;

    fn try_from(field: Field) -> Result<Self, Self::Error> {
        let attr = get_field_attr(&field)?;
        let config = get_relation_field_config(attr)?;

        Ok(RelationField {
            field,
            frontend_type: config.get(FIELD_FRONTEND_TYPE).cloned(),
            parse_with: config.get(FIELD_PARSER).cloned(),
        })
    }
}

/// Full data for public inputs.
#[derive(Clone)]
pub(super) struct PublicInputField {
    /// Common data for all inputs.
    pub inner: RelationField,
    /// The value of the `serialize_with` modifier, if any.
    pub serialize_with: Option<String>,
}

impl From<PublicInputField> for RelationField {
    fn from(public_input_field: PublicInputField) -> Self {
        public_input_field.inner
    }
}

impl TryFrom<Field> for PublicInputField {
    type Error = SynError;

    fn try_from(field: Field) -> Result<Self, Self::Error> {
        let attr = get_field_attr(&field)?;
        let config = get_public_input_field_config(attr)?;

        Ok(PublicInputField {
            inner: RelationField {
                field,
                frontend_type: config.get(FIELD_FRONTEND_TYPE).cloned(),
                parse_with: config.get(FIELD_PARSER).cloned(),
            },
            serialize_with: config.get(FIELD_SERIALIZER).cloned(),
        })
    }
}

/// The only items that will be processed from the module.
struct Items {
    struct_def: ItemStruct,
    circuit_def: ItemFn,
    imports: Vec<ItemUse>,
}

impl TryFrom<ItemMod> for IR {
    type Error = SynError;

    fn try_from(item_mod: ItemMod) -> SynResult<Self> {
        let Items {
            struct_def,
            circuit_def: circuit_definition,
            imports,
        } = extract_items(item_mod)?;

        let relation_base_name = struct_def.ident.clone();

        // Warn about items visibility.
        #[cfg(feature = "std")]
        {
            if !matches!(struct_def.vis, Visibility::Inherited) {
                eprintln!(
                    "Warning: The `{relation_base_name}` struct is public, but will be erased."
                )
            };
            if !matches!(circuit_definition.vis, Visibility::Inherited) {
                eprintln!("Warning: The circuit definition is public, but will be erased.")
            }
        }

        // Extract all fields. There should be at least one field. All fields must be named.
        let fields = match struct_def.fields {
            Fields::Named(fields) => Ok(fields),
            _ => Err(SynError::new(
                struct_def.fields.span(),
                "Expected struct with named fields",
            )),
        }?;

        // Segregate fields.
        let constants = extract_relation_fields(&fields, CONSTANT_FIELD)?;
        let public_inputs = extract_relation_fields(&fields, PUBLIC_INPUT_FIELD)?;
        let private_inputs = extract_relation_fields(&fields, PRIVATE_INPUT_FIELD)?;

        // Read field modifiers.
        let constants = cast_fields(constants)?;
        let public_inputs = cast_fields(public_inputs)?;
        let private_inputs = cast_fields(private_inputs)?;

        Ok(IR {
            relation_base_name,
            constants,
            public_inputs,
            private_inputs,
            circuit_definition,
            imports,
        })
    }
}

/// Returns the unique element from `items` that satisfies `extractor`.
///
/// `outer_span` and `item_name` are used only for error raising.
fn extract_item<I: Spanned + Clone, E: Fn(&Item) -> Option<I>>(
    items: &[Item],
    extractor: E,
    outer_span: Span,
    item_name: &'static str,
) -> SynResult<I> {
    let matching = items.iter().filter_map(extractor).collect::<Vec<_>>();
    match &*matching {
        [item] => Ok(item.clone()),
        _ => Err(SynError::new(
            outer_span,
            format!("Expected unique item: {item_name}"),
        )),
    }
}

/// Analyze `item_mod` and return only essential data from there.
fn extract_items(item_mod: ItemMod) -> SynResult<Items> {
    let items = &item_mod
        .content
        .as_ref()
        .ok_or_else(|| {
            SynError::new(
                item_mod.span(),
                "Invalid module - it is expected to be inlined",
            )
        })?
        .1;

    let span = item_mod.span();

    let relation_object_definition =
        extract_item(items, as_relation_object_def, span, "relation object")?;
    let circuit_definition = extract_item(items, as_circuit_def, span, "circuit definition")?;

    let imports = items
        .iter()
        .filter_map(|i| match i {
            Item::Use(item_use) => Some(item_use.clone()),
            _ => None,
        })
        .collect();

    Ok(Items {
        struct_def: relation_object_definition,
        circuit_def: circuit_definition,
        imports,
    })
}

/// Returns all the elements of `fields` that are attributed with `field_type`, e.g.
/// ```rust,no_run
/// #[public_input]
/// a: u8
/// ```
fn extract_relation_fields<FieldType: ?Sized>(
    fields: &FieldsNamed,
    field_type: &FieldType,
) -> SynResult<Vec<Field>>
where
    Ident: PartialEq<FieldType>,
{
    Ok(fields
        .named
        .iter()
        .filter(|f| f.attrs.iter().any(|a| a.path.is_ident(field_type)))
        .cloned()
        .collect())
}

/// Tries casting every element in `fields` into `F`.
fn cast_fields<F: TryFrom<Field, Error = SynError>>(fields: Vec<Field>) -> SynResult<Vec<F>> {
    fields
        .into_iter()
        .map(TryInto::<F>::try_into)
        .collect::<Result<Vec<_>, _>>()
}
