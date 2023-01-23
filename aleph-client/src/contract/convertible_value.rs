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

macro_rules! try_from_flat_value {
    ($ty: ty, $variant: ident, $desc: literal) => {
        impl TryFrom<ConvertibleValue> for $ty {
            type Error = anyhow::Error;

            fn try_from(value: ConvertibleValue) -> anyhow::Result<$ty> {
                match value.0 {
                    Value::$variant(value) => Ok(value.try_into()?),
                    _ => anyhow::bail!("Expected {:?} to be {}", value, $desc),
                }
            }
        }
    };
}

try_from_flat_value!(bool, Bool, "boolean");
try_from_flat_value!(char, Char, "char");
try_from_flat_value!(u16, UInt, "unsigned integer");
try_from_flat_value!(u32, UInt, "unsigned integer");
try_from_flat_value!(u64, UInt, "unsigned integer");
try_from_flat_value!(u128, UInt, "unsigned integer");
try_from_flat_value!(i16, Int, "signed integer");
try_from_flat_value!(i32, Int, "signed integer");
try_from_flat_value!(i64, Int, "signed integer");
try_from_flat_value!(i128, Int, "signed integer");

impl TryFrom<ConvertibleValue> for AccountId {
    type Error = anyhow::Error;

    fn try_from(value: ConvertibleValue) -> Result<AccountId> {
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

    fn try_from(value: ConvertibleValue) -> Result<Result<T>> {
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

    fn try_from(value: ConvertibleValue) -> Result<String> {
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

    fn try_from(value: ConvertibleValue) -> Result<Option<T>> {
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

impl<Elem: TryFrom<ConvertibleValue, Error = anyhow::Error>> TryFrom<ConvertibleValue>
    for Vec<Elem>
{
    type Error = anyhow::Error;

    fn try_from(value: ConvertibleValue) -> Result<Self> {
        let seq = match value.0 {
            Value::Seq(seq) => seq,
            _ =>  bail!("Failed parsing `ConvertibleValue` to `Vec<T>`. Expected `Seq(_)` but instead got: {:?}", value),
        };

        let mut result = vec![];
        for element in seq.elems() {
            result.push(ConvertibleValue(element.clone()).try_into()?);
        }

        Ok(result)
    }
}

impl<const N: usize, Elem: TryFrom<ConvertibleValue, Error = anyhow::Error> + Debug>
    TryFrom<ConvertibleValue> for [Elem; N]
{
    type Error = anyhow::Error;

    fn try_from(value: ConvertibleValue) -> Result<Self> {
        Vec::<Elem>::try_from(value)?
            .try_into()
            .map_err(|e| anyhow!("Failed to convert vector to an array: {e:?}"))
    }
}

#[cfg(test)]
mod tests {
    use contract_transcode::Value::{Bool, Char, Int, Seq, UInt};

    use crate::contract::ConvertibleValue;

    #[test]
    fn converts_boolean() {
        let cast: bool = ConvertibleValue(Bool(true))
            .try_into()
            .expect("Should cast successfully");
        assert!(cast);
    }

    #[test]
    fn converts_char() {
        let cast: char = ConvertibleValue(Char('x'))
            .try_into()
            .expect("Should cast successfully");
        assert_eq!('x', cast);
    }

    #[test]
    fn converts_biguint() {
        let long_uint = 41414141414141414141414141414141414141u128;
        let cast: u128 = ConvertibleValue(UInt(long_uint))
            .try_into()
            .expect("Should cast successfully");
        assert_eq!(long_uint, cast);
    }

    #[test]
    fn converts_uint() {
        let cast: u32 = ConvertibleValue(UInt(41))
            .try_into()
            .expect("Should cast successfully");
        assert_eq!(41, cast);
    }

    #[test]
    fn converts_bigint() {
        let long_int = -41414141414141414141414141414141414141i128;
        let cast: i128 = ConvertibleValue(Int(long_int))
            .try_into()
            .expect("Should cast successfully");
        assert_eq!(long_int, cast);
    }

    #[test]
    fn converts_int() {
        let cast: i32 = ConvertibleValue(Int(-41))
            .try_into()
            .expect("Should cast successfully");
        assert_eq!(-41, cast);
    }

    #[test]
    fn converts_integer_array() {
        let cv = ConvertibleValue(Seq(vec![UInt(4), UInt(1)].into()));
        let cast: [u32; 2] = cv.try_into().expect("Should cast successfully");
        assert_eq!([4u32, 1u32], cast);
    }

    #[test]
    fn converts_integer_sequence() {
        let cv = ConvertibleValue(Seq(vec![UInt(4), UInt(1)].into()));
        let cast: Vec<u32> = cv.try_into().expect("Should cast successfully");
        assert_eq!(vec![4u32, 1u32], cast);
    }

    #[test]
    fn converts_nested_sequence() {
        let words = vec![
            vec!['s', 'u', 'r', 'f', 'i', 'n'],
            vec![],
            vec!['b', 'i', 'r', 'd'],
        ];
        let encoded_words = words
            .iter()
            .map(|word| Seq(word.iter().cloned().map(Char).collect::<Vec<_>>().into()))
            .collect::<Vec<_>>();

        let cv = ConvertibleValue(Seq(encoded_words.into()));
        let cast: Vec<Vec<char>> = cv.try_into().expect("Should cast successfully");

        assert_eq!(words, cast);
    }
}
