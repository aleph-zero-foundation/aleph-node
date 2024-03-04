# pallet-operations

General ChainOps extrinsic that are used in chain maintenance activities.

## fix_accounts_consumers_underflow

An account can have an underflow of a `consumers` counter. 
Account categories that are impacted by this issue depends on a chain runtime,
but specifically for AlephNode runtime are as follows:
* `consumers`  == 0, `reserved`  > 0
* `consumers`  == 1, `balances.Locks` contain an entry with `id`  == `vesting`
* `consumers`  == 2, `balances.Locks` contain an entry with `id`  == `staking`
* `consumers`  == 3, `balances.Locks` contain entries with `id`  == `staking`
   and account id is in `session.nextKeys`

`fix_accounts_consumers_underflow` checks if the account falls into one of above
categories, and increase its `consumers` counter.

