import logging
from aleph_chain_version import AlephChainVersion
from utils import format_balance, save_accounts_to_json_file
from consumers_counter import get_expected_consumers_counter

log = logging.getLogger()


def check_account_balances_invariants(account, chain_major_version, ed):
    """
    This predicate checks whether an accounts meet pallet balances and general account reference counters predicates.

    :param account: AccountInfo struct (element of System.Accounts StorageMap)
    :param chain_major_version: integer which is major version of AlephNode chain
    :param ed: existential deposit
    :return: True if account meets all invariants, False otherwise
    """
    providers = account['providers']
    consumers = account['consumers']
    free = account['data']['free']
    reserved = account['data']['reserved']

    # in both versions, consumers must be 0 if providers are 0; also there is only one provider which is pallet
    # balance so max possible value of providers is 1
    account_ref_counter_invariant = (providers <= 1 and consumers == 0) or (consumers > 0 and providers == 1)

    if chain_major_version <= AlephChainVersion.VERSION_11_4:
        misc_frozen = account['data']['misc_frozen']
        fee_frozen = account['data']['fee_frozen']

        # in pre-12 version, existential deposit applies to total balance
        ed_is_for_total_balance_invariant = free + reserved >= ed

        # in pre-12 version, locked balance applies only to free balance
        locked_balance_is_on_free_balance_invariant = free >= max(misc_frozen, fee_frozen)

        return account_ref_counter_invariant and \
            ed_is_for_total_balance_invariant and \
            locked_balance_is_on_free_balance_invariant

    frozen = account['data']['frozen']
    flags = account['data']['flags']

    # in at least 12 version, ED must be available on free balance for account to exist
    ed_is_for_free_balance_only_invariant = free >= ed

    # in at least 12 version, locked balance applies to total balance
    locked_balance_is_on_total_balance_invariant = free + reserved >= frozen

    # all accounts must be upgraded already
    is_account_already_upgraded = flags >= 2 ** 127

    return \
            account_ref_counter_invariant and \
            ed_is_for_free_balance_only_invariant and \
            locked_balance_is_on_total_balance_invariant and \
            is_account_already_upgraded


def perform_accounts_state_checks(chain_connection,
                                  accounts,
                                  chain_major_version,
                                  locks,
                                  bonded,
                                  ledger,
                                  next_keys,
                                  contract_accounts,
                                  block_hash):
    """
    Checks whether all accounts on a chain matches pallet balances invariants and
    consumers counter has expected value
    :param chain_connection: WS connection handler
    :param accounts: A list of tuples (AccountId, AccountInfo)
    :param chain_major_version: enum ChainMajorVersion
    :param locks: Balances.Locks storage map
    :param bonded: Staking.Bonded storage map
    :param ledger: Staking.Ledger storage map
    :param next_keys: Session.NextKeys storage map
    :param contract_accounts: Contracts.ContractInfoOf storage map
    :param block_hash: A block hash to query state from
    :return:None
    """
    existential_deposit = chain_connection.get_constant(module_name="Balances",
                                                        constant_name="ExistentialDeposit",
                                                        block_hash=block_hash).value
    log.info(f"Existential deposit is {format_balance(chain_connection, existential_deposit)}")
    log.info(f"Checking pallet balances invariants...")
    accounts_failed_balances_invariants = list(filter(
        lambda account_and_info: not check_account_balances_invariants(account_and_info[1],
                                                                       chain_major_version,
                                                                       existential_deposit),
        accounts))

    if len(accounts_failed_balances_invariants) > 0:
        log.warning(f"Found {len(accounts_failed_balances_invariants)} accounts that do not "
                    f"meet balances invariants!")

    if chain_major_version >= AlephChainVersion.VERSION_13_3:
        log.info(f"Checking consumers counter...")
        accounts_with_wrong_consumers_counter = list(filter(
            lambda account_and_info: not check_consumers_counter(chain_major_version,
                                                                 account_and_info[0],
                                                                 account_and_info[1],
                                                                 locks,
                                                                 bonded,
                                                                 ledger,
                                                                 next_keys,
                                                                 contract_accounts),
            accounts))
        if len(accounts_with_wrong_consumers_counter) > 0:
            log.warning(f"Found {len(accounts_with_wrong_consumers_counter)} accounts that have "
                        f"incorrect consumers counter!")
            accounts_failed_balances_invariants.extend(accounts_with_wrong_consumers_counter)

    if len(accounts_failed_balances_invariants) > 0:
        log.warning(f"There are {len(accounts_failed_balances_invariants)} accounts with failed invariants!")
        save_accounts_to_json_file("accounts-with-failed-invariants.json", accounts_failed_balances_invariants)
    else:
        log.info(f"All accounts on chain {chain_connection.chain} meet balances invariants.")


def check_consumers_counter(chain_major_version,
                            account_id,
                            account_info,
                            locks,
                            bonded,
                            ledger,
                            next_keys,
                            contract_accounts):
    """
    This predicate checks whether account consumers counter has an expected value.
    :param chain_major_version: enum ChainMajorVersion
    :param account_id: AccountId
    :param account_info: AccountInfo struct (element of System.Accounts StorageMap)
    :param locks: Balances.Locks storage map
    :param bonded: Staking.Bonded storage map
    :param ledger: Staking.Ledger storage map
    :param next_keys: Session.NextKeys storage map
    :param contract_accounts: Contracts.ContractInfoOf storage map
    :return: True if account has correct consumers counter, False otherwise
    """
    assert chain_major_version >= AlephChainVersion.VERSION_13_3, \
        f"You must run this on AlephNode chain with at least 13.3 version!"

    expected_consumers_counter = get_expected_consumers_counter(chain_major_version,
                                                                account_id,
                                                                account_info,
                                                                locks,
                                                                bonded,
                                                                ledger,
                                                                next_keys,
                                                                contract_accounts)
    current_consumers_counter = account_info['consumers']
    log.debug(f"Account {account_id}, expected consumers counter: {expected_consumers_counter}, "
              f"current consumers counter: {current_consumers_counter}")
    return expected_consumers_counter == current_consumers_counter
