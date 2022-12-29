use std::{fmt::Debug, ops::Deref, str::FromStr};

use anyhow::{anyhow, bail, Context, Result};
use contract_transcode::Value;

use crate::AccountId;

/// Temporary wrapper for converting from [Value] to primitive types.
///
/// ```
/// # #![feature(assert_matches)]
/// # #![feature(type_ascription)]
/// # use std::assert_matches::assert_matches;
/// # use anyhow::{anyhow, Result};
/// # use aleph_client::{AccountId, contract::ConvertibleValue};
/// use contract_transcode::Value;
///
/// assert_matches!(ConvertibleValue(Value::UInt(42)).try_into(), Ok(42u128));
/// assert_matches!(ConvertibleValue(Value::UInt(42)).try_into(), Ok(42u32));
/// assert_matches!(ConvertibleValue(Value::UInt(u128::MAX)).try_into(): Result<u32>, Err(_));
/// assert_matches!(ConvertibleValue(Value::Bool(true)).try_into(), Ok(true));
/// assert_matches!(
///     ConvertibleValue(Value::Literal("5H8cjBBzCJrAvDn9LHZpzzJi2UKvEGC9VeVYzWX5TrwRyVCA".to_string())).
///         try_into(): Result<AccountId>,
///     Ok(_)
/// );
/// assert_matches!(
///     ConvertibleValue(Value::String("not a number".to_string())).try_into(): Result<u128>,
///     Err(_)
/// );
/// ```
#[derive(Debug, Clone)]
pub struct ConvertibleValue(pub Value);

impl Deref for ConvertibleValue {
    type Target = Value;

    fn deref(&self) -> &Value {
        &self.0
    }
}

impl TryFrom<ConvertibleValue> for bool {
    type Error = anyhow::Error;

    fn try_from(value: ConvertibleValue) -> Result<bool, Self::Error> {
        match value.0 {
            Value::Bool(value) => Ok(value),
            _ => bail!("Expected {:?} to be a boolean", value.0),
        }
    }
}

impl TryFrom<ConvertibleValue> for u128 {
    type Error = anyhow::Error;

    fn try_from(value: ConvertibleValue) -> Result<u128, Self::Error> {
        match value.0 {
            Value::UInt(value) => Ok(value),
            _ => bail!("Expected {:?} to be an integer", value.0),
        }
    }
}

impl TryFrom<ConvertibleValue> for u32 {
    type Error = anyhow::Error;

    fn try_from(value: ConvertibleValue) -> Result<u32, Self::Error> {
        match value.0 {
            Value::UInt(value) => Ok(value.try_into()?),
            _ => bail!("Expected {:?} to be an integer", value.0),
        }
    }
}

impl TryFrom<ConvertibleValue> for AccountId {
    type Error = anyhow::Error;

    fn try_from(value: ConvertibleValue) -> Result<AccountId, Self::Error> {
        match value.0 {
            Value::Literal(value) => {
                AccountId::from_str(&value).map_err(|_| anyhow!("Invalid account id"))
            }
            _ => bail!("Expected {:?} to be a string", value),
        }
    }
}

impl<T> TryFrom<ConvertibleValue> for Result<T>
where
    ConvertibleValue: TryInto<T, Error = anyhow::Error>,
{
    type Error = anyhow::Error;

    fn try_from(value: ConvertibleValue) -> Result<Result<T>, Self::Error> {
        if let Value::Tuple(tuple) = &value.0 {
            match tuple.ident() {
                Some(x) if x == "Ok" => {
                    if tuple.values().count() == 1 {
                        let item =
                            ConvertibleValue(tuple.values().next().unwrap().clone()).try_into()?;
                        return Ok(Ok(item));
                    } else {
                        bail!("Unexpected number of elements in Ok variant: {:?}", &value);
                    }
                }
                Some(x) if x == "Err" => {
                    if tuple.values().count() == 1 {
                        return Ok(Err(anyhow!(value.to_string())));
                    } else {
                        bail!("Unexpected number of elements in Err variant: {:?}", &value);
                    }
                }
                _ => (),
            }
        }

        bail!("Expected {:?} to be an Ok(_) or Err(_) tuple.", value);
    }
}

impl TryFrom<ConvertibleValue> for String {
    type Error = anyhow::Error;

    fn try_from(value: ConvertibleValue) -> std::result::Result<String, Self::Error> {
        let seq = match value.0 {
            Value::Seq(seq) => seq,
            _ =>  bail!("Failed parsing `ConvertibleValue` to `String`. Expected `Seq(Value::UInt)` but instead got: {:?}", value),
        };

        let mut bytes: Vec<u8> = Vec::with_capacity(seq.len());
        for el in seq.elems() {
            if let Value::UInt(byte) = *el {
                if byte > u8::MAX as u128 {
                    bail!("Expected number <= u8::MAX but instead got: {:?}", byte)
                }
                bytes.push(byte as u8);
            } else {
                bail!("Failed parsing `ConvertibleValue` to `String`. Expected `Value::UInt` but instead got: {:?}", el);
            }
        }
        String::from_utf8(bytes).context("Failed parsing bytes to UTF-8 String.")
    }
}

auto trait NotEq {}
// We're basically telling the compiler that there is no instance of NotEq for `(X,X)` tuple.
// Or put differently - that you can't implement `NotEq` for `(X,X)`.
impl<X> !NotEq for (X, X) {}

impl<T> TryFrom<ConvertibleValue> for Option<T>
where
    T: TryFrom<ConvertibleValue, Error = anyhow::Error> + Debug,
    // We will derive this impl only when `T != ConvertibleValue`.
    // Otherwise we will get a conflict with generic impl in the rust `core` crate.
    (ConvertibleValue, T): NotEq,
{
    type Error = anyhow::Error;

    fn try_from(value: ConvertibleValue) -> std::result::Result<Option<T>, Self::Error> {
        let tuple = match &value.0 {
            Value::Tuple(tuple) => tuple,
            _ => bail!("Expected {:?} to be a Some(_) or None Tuple.", &value),
        };

        match tuple.ident() {
            Some(x) if x == "Some" => {
                if tuple.values().count() == 1 {
                    let item =
                        ConvertibleValue(tuple.values().next().unwrap().clone()).try_into()?;
                    Ok(Some(item))
                } else {
                    bail!(
                        "Unexpected number of elements in Some(_) variant: {:?}. Expected one.",
                        &value
                    );
                }
            }
            Some(x) if x == "None" => {
                if tuple.values().count() == 0 {
                    Ok(None)
                } else {
                    bail!(
                        "Unexpected number of elements in None variant: {:?}. Expected zero.",
                        &value
                    );
                }
            }
            _ => bail!(
                "Expected `.ident()` to be `Some` or `None`, got: {:?}",
                &tuple
            ),
        }
    }
}
