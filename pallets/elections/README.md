# pallet-elections

This pallet manages changes in the committee responsible for producing blocks and establishing consensus.

## Terminology
For definition of session, era, staking see pallet_session and pallet_staking.
- committee ([`EraValidators`]): Set of nodes that produce and finalize blocks in the session.
- validator: Node that can become a member of committee (or already is) via rotation.
- `EraValidators::reserved`: immutable validators, ie they cannot be removed from that list.
- `EraValidators::non_reserved`: validators that can be banned out from that list.

## Elections process
There are two options for choosing validators during election process governed by ([`Openness`]) storage value:
- `Permissionless`: choose all validators that bonded enough amount and are not banned.
- `Permissioned`: choose `EraValidators::reserved` and all `EraValidators::non_reserved` that are not banned.

License: Apache 2.0
