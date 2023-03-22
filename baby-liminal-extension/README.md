# BabyLiminal chain extension

This crate is an implementation of BabyLiminal chain extension, with both ink! and Substrate counterparts available.

## ink!

### Usage

To use `baby-liminal-extension` with ink!, include it as a dependency and activate `ink-std` 
feature when `std` feature of your contract is enabled:

```toml
baby-liminal-extension = { path = "...", default-features = false, features = ["ink"] }

# ...

[features]
# ...
std = [
    # ...
    "baby-liminal-extension/ink-std"
]
```

Next, simply call `BabyLiminalExtension` methods on `baby_liminal_extension::ink::Extension`:

```rust
use baby_liminal_extension::{ink::Extension, BabyLiminalExtension};

Extension.store_key(...);
```

### Testing

To test chain extension with `ink` features enabled, you have to ensure that you removed any other mention of `baby-liminal-extension`
with `substrate` feature enabled, otherwise `rustc` will emit errors related to duplicated items.

For example, you can comment out `baby-liminal-extension` mentions from `runtime` crate, then try to run
the necessary checks/tests in `baby-liminal-extension` directory.

## Substrate

### Usage

To use `baby-liminal-extension` with Substrate, add `baby_liminal_extension::substrate::Extension` to `pallet_contracts::Config`'s `ChainExtension` associated type:

```rust
impl pallet_contracts::Config for Runtime {
    // ...
    type ChainExtension = baby_liminal_extension::substrate::Extension;
}
```

### Implementation details

`baby-liminal-extension` introduces several types for you to use during the
chain extension development/usage.

#### `BabyLiminalExtension`

The trait, thanks to being marked with `#[obce::definition]`, provides a description
of what chain extension does, as well as contains an automatically generated `obce::codegen::ExtensionDescription`
and `obce::codegen::MethodDescription` trait impls.

#### `BabyLiminalError`

`BabyLiminalError` is a type that describes all errors that can occur during chain extension calls.

Using `#[obce::error]` attribute, accompanied by `ret_val` variant attributes,
`BabyLiminalError` has an automatically implemented common traits (`Copy`, `Clone`, `scale::Encode`, `scale::Decode`, etc.) for the type itself, and an implementation of `TryFrom<BabyLiminalError>` for `pallet_contracts::chain_extension::RetVal`. This implementation allows us to automatically convert `BabyLiminalError` to `RetVal` if `#[obce::implementation]` methods have `ret_val` attribute on them.

#### `substrate` module

The `substrate` module contains the chain extension implementation itself.

Every method is marked with with `#[obce(weight(expr = "...", pre_charge = true), ret_val)]`,
meaning that they:

1. Pre-charge weight calculated by the provided expression.
2. Return a `Result<T, E>`, that has to be converted to `pallet_contracts::chain_extension::RetVal` if possible (in our case, since `BabyLiminalError` has all variants attributed with `#[obce(ret_val = "...")]` we simply convert every error instance to `RetVal`).

An additional `Env: Executor<T>` bound exists on `impl` block to mock pallet calls
with `Executor` trait available at `executor` module.

#### Testing

For mocking chain extension environment there exists a `MockedEnvironment` struct, which
implements both `obce::substrate::ChainExtensionEnvironment` and `Executor` traits,
which change their behaviors depending on the passed const generics values.

Various utility types (like `StoreKeyOkayer` or `VerifyErrorer`) exist to simplify
chain extension testing even further by providing a simple const generics-based interface to
configure testing environment.

Method identifier constants (`STORE_KEY_ID` and `VERIFY_ID`) were acquired by expanding
macros using `cargo expand`, and are depending solely on the method names (thus making them stable between compilations).
