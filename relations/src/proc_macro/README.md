# liminal-ark-relation-macro

This crate provides `snark_relation` procedural macro for concise defining SNARK relations.
Given minimal relation definition, this macro will generate all the required code for creating and casting partial relation objects, public input serialization and circuit generation.

## General usage

The `#[snark_relation]` attribute is intended for modules. Such module must define two items:
  1. *relation object*: the collection of all constant, public and private relation data. The
     struct must be defined with `#[relation_object_definition]` attribute. All other attributes
     will be preserved.
  2. *circuit definition*: the circuit form. The function must be defined with
     `#[circuit_definition]` attribute. The signature can be arbitrary: function body will be
     used in `ark_relations::r1cs::ConstraintSynthesizer` trait implementation. All function
     attributes (like feature-gating or linting) are preserved and added at the `impl` level.

Provided with these inputs, the macro will generate following items (outside the module).
  -  Three new public structs: `<R>WithoutInput`, `<R>WithPublicInput` and `<R>WithFullInput`,
     where `<R>` is the name of the relation object struct. The first one will have only
     constants as its fields, the second one will have additionally public inputs, and the last
     one will have all the data.
  -  `new(..)` constructors for every struct. **Important**: the order of constructor arguments
     is: all the constants, then public inputs, and at the end private inputs. The order in each
     group is inherited from the relation object definition.
  -  Getters for the fields. For constants, the signature is `fn <field>(&self) ->
     &<field_type>`. For public and private inputs, the signature is `fn <field>(&self) ->
     Result<&<field_type>, SynthesisError>`. All the structs have the same set of getters.
     When a field is missing, `SynthesisError::MissingAssignment` is returned.
  -  Conversions from `<R>WithFullInput` to `<R>WithPublicInput` and from `<R>WithPublicInput` to `<R>WithoutInput`.
  -  A `serialize_public_input(&self)` method for `<R>WithPublicInput`.
  -  Implementation of `ConstraintSynthesizer` trait for `<R>WithoutInput` (with setup mode check).
  -  Implementation of `ConstraintSynthesizer` trait for `<R>WithFullInput`.

```rust
#[snark_relation]
mod relation {
    #[relation_object_definition]
    struct SomeRelation {
        #[constant]
        a: CF,
        #[public_input]
        b: CF,
        #[private_input]
        c: CF,
    }

    #[circuit_definition]
    fn generate_circuit() -> ark_relations::r1cs::Result<()> {
        Ok(())
    }
}
```

All the imports (`use` items) that are present in the module will be copied and moved outside (together with the generated items).

## Field attributes

Fields can have additional modifiers. Constants and private inputs can be enriched with:
  -  *frontend type* (e.g. `#[private_input(frontend_type = "u32")]`) - this specifies what type
     should be expected in the constructors. The item type (the backend one) will be then created
     from frontend value and used later on.
  -  *frontend value parser* (e.g. `#[private_input(frontend_type = "u32", parse_with =
     "u32_to_CF")]`) - this is the method that will be used for translating frontend value to the
     backend type in the constructors. Unless specified, `.into()` will be used. It cannot be
     used without `frontend_type`.

Public inputs can have one more modifier:
  -  *serializator* (e.g. `#[public_input(serialize_with = "flatten_sequence")]`) - the
     serialization process should result in `Vec<CF>` (where `CF` is the circuit field type). By
     default, every public input will be firstly wrapped into a singleton vector (`vec![input]`),
     and then, the ordered results will be flattened with `.concat()`. In case your input
     requires some other way to fit into (usually flattening), you can pass you custom
     serializator.

All the values in modifiers (function names, types) must be passed as string literals (within `""`).

 ```rust
use ark_std::{One, Zero};
use snark_relation_proc_macro::snark_relation;

use crate::CircuitField;

fn parse_u16(x: u16) -> CircuitField {
    CircuitField::from(x)
}

fn byte_to_bits<F: Zero + One + Copy>(byte: &u8) -> Vec<F> {
    let mut bits = [F::zero(); 8];
    for (idx, bit) in bits.iter_mut().enumerate() {
        if (byte >> idx) & 1 == 1 {
            *bit = F::one();
        }
    }
    bits.to_vec()
}

#[snark_relation]
mod relation {
    #[relation_object_definition]
    struct SomeRelation {
        #[constant]
        a: u8,
        #[public_input(frontend_type = "u16", parse_with = "parse_u16")]
        b: CF,
        #[private_input(frontend_type = "u32")]
        c: u64,
        #[public_input(serialize_with = "byte_to_bits")]
        d: u8,
    }

    #[circuit_definition]
    fn generate_circuit() -> ark_relations::r1cs::Result<()> {
        Ok(())
    }
}
 ```
