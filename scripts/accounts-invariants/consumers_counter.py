import logging
from aleph_chain_version import AlephChainVersion
from chain_operations import query_storage_map

log = logging.getLogger()


def reserved_or_frozen_non_zero(account_id, account_info):
    """
    Checks if an account has zero consumers, but non-zero reserved amount.
    """
    reserved = account_info['data']['reserved']
    frozen = account_info['data']['frozen']
    if frozen > 0 or reserved > 0:
        log.debug(f"Account {account_id} has non-zero reserved or non-zero frozen.")
        return True
    return False


def get_staking_lock(account_id_locks):
    staking_lock_encoded_id = "0x7374616b696e6720"
    return next(filter(lambda lock: lock['id'] == staking_lock_encoded_id, account_id_locks), None)


def get_vesting_lock(account_id_locks):
    vesting_lock_encoded_id = "0x76657374696e6720"
    vesting_lock = next(filter(lambda lock: lock['id'] == vesting_lock_encoded_id, account_id_locks), None)
    return vesting_lock is not None


def query_staking_data(chain_connection, block_hash):
    """
    Retrieves chain data that is required to calculate correct consumers counter for an account.
     :param chain_connection: WS connection handler
     :param block_hash: A block hash to query state from
    """
    next_keys = set()
    query_storage_map(chain_connection=chain_connection,
                      pallet="Session",
                      storage_map="NextKeys",
                      block_hash=block_hash,
                      output_container=next_keys,
                      output_functor=lambda account_id, storage_value, output_container:
                      output_container.add(account_id.value))

    locks = {}
    query_storage_map(chain_connection=chain_connection,
                      pallet="Balances",
                      storage_map="Locks",
                      block_hash=block_hash,
                      output_container=locks,
                      output_functor=lambda account_id, storage_value, output_container:
                      output_container.update({account_id.value: storage_value.value})
                      )

    bonded = {}
    query_storage_map(chain_connection=chain_connection,
                      pallet="Staking",
                      storage_map="Bonded",
                      block_hash=block_hash,
                      output_container=bonded,
                      output_functor=lambda account_id, storage_value, output_container:
                      output_container.update({account_id.value: storage_value.value})
                      )

    ledgers = {}
    query_storage_map(chain_connection=chain_connection,
                      pallet="Staking",
                      storage_map="Ledger",
                      block_hash=block_hash,
                      output_container=ledgers,
                      output_functor=lambda account_id, storage_value, output_container:
                      output_container.update({account_id.value: storage_value.serialize()['stash']})
                      )

    return bonded, ledgers, locks, next_keys


def is_stash_equal_to_controller_and_in_next_keys(account_id, bonded, next_keys):
    if bonded[account_id] == account_id and account_id in next_keys:
        log.debug(f"Found an account that has next session key, and that account's stash == controller: {account_id}")
        return True
    return False


def get_expected_consumers_counter(chain_major_version,
                                   account_id,
                                   account_info,
                                   locks,
                                   bonded,
                                   ledger,
                                   next_keys,
                                   contract_accounts):
    """
    Returns expected consumers counter
    :param chain_major_version: enum ChainMajorVersion
    :param account_id: AccountId
    :param account_info: AccountInfo struct (element of System.Accounts StorageMap)
    :param locks: Balances.Locks storage map
    :param bonded: Staking.Bonded storage map
    :param ledger: Staking.Ledger storage map
    :param next_keys: Session.NextKeys storage map
    :param contract_accounts: Contracts.ContractInfoOf storage map
    :return: Integer denoting consumers ccounter
    """
    assert chain_major_version >= AlephChainVersion.VERSION_13_3, \
        f"You must run this on AlephNode chain with at least 13.3 version!"

    expected_consumers = 0
    if reserved_or_frozen_non_zero(account_id, account_info):
        expected_consumers += 1
    if is_account_bonded(account_id, bonded):
        expected_consumers += 1
        if is_stash_equal_to_controller_and_in_next_keys(account_id, bonded, next_keys):
            expected_consumers += 1
    if is_controller_account_and_stash_is_different(account_id,
                                                    ledger,
                                                    next_keys):
        expected_consumers += 1
    if is_contract_account(account_id, contract_accounts):
        expected_consumers += 1
    if chain_major_version == AlephChainVersion.VERSION_13_3:
        # locks contribute to consumers counter only in AlephNode <= 13 version
        if has_at_least_one_lock(account_id, locks):
            expected_consumers += 1
    return expected_consumers


def is_controller_account_and_stash_is_different(account_id, ledger, next_keys):
    if account_id in ledger:
        stash_account = ledger[account_id]
        if stash_account != account_id:
            if stash_account in next_keys:
                log.debug(f"Found a controller account {account_id}, which has different stash: {stash_account}"
                          f" and {stash_account} is in the next keys")
                return True
    return False


def is_account_bonded(account_id, bonded):
    if account_id in bonded:
        log.debug(f"Account {account_id} is bonded")
        return True
    return False


def is_contract_account(account_id, contract_accounts):
    if account_id in contract_accounts:
        log.debug(f"Found a contract account: {account_id}")
        return True
    return False


def has_at_least_one_lock(account_id, locks):
    if account_id in locks and len(locks[account_id]) > 0:
        log.debug(f"Account {account_id} has following locks: {locks[account_id]}")
        has_vesting_lock = get_vesting_lock(locks[account_id]) is not None
        has_staking_lock = get_staking_lock(locks[account_id]) is not None
        return has_vesting_lock or has_staking_lock
    return False
