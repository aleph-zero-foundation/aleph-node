# liminal-ark-relations

This is a library containing a couple of R1CS relations.
It was built using [arkworks](https://github.com/arkworks-rs) libraries.

## Provided relations
1. `xor` - representing `a ⊕ b = c`
2. `linear-equation` - representing `a·x + b = y`
3. `preimage` - representing Poseidon 1:1 hashing
4. `deposit`, `deposit-and-merge`, `merge` and `withdraw` relations that are used in Shielder zk-app (see: https://github.com/cardinal-Cryptography/zk-apps/)

All relations were built using [`liminal-ark-relation-macro`](https://crates.io/crates/liminal-ark-relation-macro).
