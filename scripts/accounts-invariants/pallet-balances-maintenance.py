#!/bin/python3

from chain_operations import *
from aleph_chain_version import *
from utils import *
from total_issuance import *

import substrateinterface
import argparse
import os
import logger
import logging
import contracts
from tqdm import tqdm
import sys


def get_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="""
Script for maintenance operations on AlephNode chain with regards to pallet balances.

It has following functionality: 
* checking pallet balances and account reference counters invariants.
* calling pallet operations for maintenance actions
* checking total issuance 

By default, it connects to a AlephZero Testnet and performs sanity checks only ie not changing state of the chain at all.
Accounts that do not satisfy those checks are written to accounts-with-failed-invariants.json file.
""",
        formatter_class=argparse.RawTextHelpFormatter)

    parser.add_argument('--ws-url',
                        type=str,
                        default='wss://ws.test.azero.dev:443',
                        help='WS URL of the RPC node to connect to. Default is wss://ws.test.azero.dev:443')
    parser.add_argument('--log-level',
                        default='info',
                        choices=['debug', 'info', 'warning', 'error'],
                        help='Provide global logging level, for both file and console logger. If set to debug, '
                             'file logger will log with debug anyway, but console with info. If set to info, '
                             'both will log as info.')
    parser.add_argument('--dry-run',
                        action='store_true',
                        help='Specify this switch if script should just print what if would do. Default: False')
    parser.add_argument('--fix-consumers-calls-in-batch',
                        type=int,
                        default=10,
                        help='How many consumers underflow accounts to fix in one script run session.'
                             ' Default: 10')
    parser.add_argument('--block-hash',
                        type=str,
                        default='',
                        help='Block hash from which this script should query state from. Default: chain tip.')
    parser.add_argument('--start-range-block-hash',
                        type=str,
                        default='',
                        help='A block hash which denotes starting interval when searching for total issuance '
                             'imbalance. Must be present when --check-total-issuance is set.')
    parser.add_argument('--check-total-issuance',
                        action='store_true',
                        help='Specify this switch if script should compare total issuance aggregated over all '
                             'accounts with StorageValue balances.total_issuance')
    group = parser.add_mutually_exclusive_group()
    group.add_argument('--fix-consumers-counter-underflow',
                       action='store_true',
                       help='Specify this switch if script should query all accounts that have underflow consumers '
                            'counter, and fix consumers counter by issuing batch calls of '
                            'Operations.fix_accounts_consumers_underflow call.'
                            'Query operation is possible on all chains, but fix only on AlephNode >= 13.2'
                            'Default: False')
    return parser.parse_args()


def check_account_invariants(account, chain_major_version, ed):
    """
    This predicate checks whether an accounts meet pallet balances and account reference counters predicates.

    :param account: AccountInfo struct (element of System.Accounts StorageMap)
    :param chain_major_version: integer which is major version of AlephNode chain
    :param ed: existential deposit
    :return: True if account meets all invariants, False otherwise
    """
    providers = account['providers'].value
    consumers = account['consumers'].value
    free = account['data']['free'].value
    reserved = account['data']['reserved'].value

    # in both versions, consumers must be 0 if providers are 0; also there is only one provider which is pallet
    # balance so max possible value of providers is 1
    account_ref_counter_invariant = (providers <= 1 and consumers == 0) or (consumers > 0 and providers == 1)

    if chain_major_version <= AlephChainVersion.VERSION_11_4:
        misc_frozen = account['data']['misc_frozen'].value
        fee_frozen = account['data']['fee_frozen'].value

        # in pre-12 version, existential deposit applies to total balance
        ed_is_for_total_balance_invariant = free + reserved >= ed

        # in pre-12 version, locked balance applies only to free balance
        locked_balance_is_on_free_balance_invariant = free >= max(misc_frozen, fee_frozen)

        return account_ref_counter_invariant and \
               ed_is_for_total_balance_invariant and \
               locked_balance_is_on_free_balance_invariant

    frozen = account['data']['frozen'].value
    flags = account['data']['flags'].value

    # in at least 12 version, ED must be available on free balance for account to exist
    ed_is_for_free_balance_only_invariant = free >= ed

    # in at least 12 version, locked balance applies to total balance
    locked_balance_is_on_total_balance_invariant = free + reserved >= frozen

    is_account_already_upgraded = flags >= 2 ** 127
    # the reasons we check if account is upgraded only in this check and not in the previous invariants is that
    # * ed_is_for_free_balance_only_invariant is stricter than ed_is_for_total_balance_invariant and account
    #   account upgrade code has a bug so it does not provide ed for accounts which does not meet this in 11 version
    # * locked_balance_is_on_total_balance_invariant is less strict than locked_balance_is_on_free_balance_invariant
    # * consumer_ref_applies_to_suspended_balances_invariant applies to both versions
    consumer_ref_applies_to_suspended_balances_invariant = \
        (not is_account_already_upgraded or (frozen == 0 and reserved == 0) or consumers > 0)
    return \
        account_ref_counter_invariant and \
        ed_is_for_free_balance_only_invariant and \
        locked_balance_is_on_total_balance_invariant and \
        consumer_ref_applies_to_suspended_balances_invariant


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


def has_account_consumer_underflow(account_id_and_info, locks, bonded, ledger, next_keys, contract_accounts):
    """
    Returns True if an account has consumers underflow
    """
    account_id, account_info = account_id_and_info
    current_consumers = account_info['consumers']
    expected_consumers = get_expected_consumers_counter_in_13_version(account_id,
                                                                      account_info,
                                                                      locks,
                                                                      bonded,
                                                                      ledger,
                                                                      next_keys,
                                                                      contract_accounts)
    if current_consumers < expected_consumers:
        log.debug(f"Current consumers counter {current_consumers} is less than expected consumers counter "
                  f"{expected_consumers}, performing migration for account {account_id}")
        return True
    logging.debug(
        f"Account {account_id} current_consumers counter: {current_consumers}, expected: {expected_consumers}")
    return False


def get_expected_consumers_counter_in_13_version(account_id,
                                                 account_info,
                                                 locks,
                                                 bonded,
                                                 ledger,
                                                 next_keys,
                                                 contract_accounts):
    expected_consumers = 0
    if reserved_or_frozen_non_zero(account_id, account_info):
        expected_consumers += 1
    if account_id in locks and len(locks[account_id]) > 0:
        log.debug(f"Account {account_id} has following locks: {locks[account_id]}")
        has_vesting_lock = get_vesting_lock(locks[account_id]) is not None
        has_staking_lock = get_staking_lock(locks[account_id]) is not None
        if has_vesting_lock or has_staking_lock:
            expected_consumers += 1
            if has_staking_lock:
                expected_consumers += 1
    if account_id in next_keys and \
            account_id in bonded and bonded[account_id] == account_id:
        log.debug(f"Found an account that has next session key, and that account's stash == controller: {account_id}")
        expected_consumers += 1
    if account_id in ledger:
        stash_account = ledger[account_id]
        if stash_account != account_id:
            log.debug(f"Found a controller account {account_id}, which has different stash: {stash_account}")
            if stash_account in next_keys:
                expected_consumers += 1
    if account_id in contract_accounts:
        log.debug(f"Found a contract account: {account_id}")
        # TODO uncomment when pallet operations is able to query contract accounts
        # expected_consumers += 1
    return expected_consumers


def query_accounts_with_consumers_counter_underflow(chain_connection, block_hash=None):
    """
    Queries all accounts that have an underflow of consumers counter by calculating expected number of consumers and
    comparing to current consumers counter

    :param chain_connection: WS connection handler
    :param block_hash: A block hash to query state from
    """
    bonded, ledger, locks, next_keys = get_consumers_related_data(chain_connection, block_hash)
    _, contract_accounts = contracts.query_contract_and_code_owners_accounts(
        chain_connection=chain_ws_connection,
        block_hash=block_hash)

    log.info("Querying all accounts and filtering by consumers underflow predicate.")
    return [account_id_and_info for account_id_and_info in get_all_accounts(chain_connection, block_hash) if
            has_account_consumer_underflow(account_id_and_info, locks, bonded, ledger, next_keys, contract_accounts)]


def get_consumers_related_data(chain_connection, block_hash=None):
    """
    Retrieves chain data that relates to manipulating consumers counter.
     :param chain_connection: WS connection handler
     :param block_hash: A block hash to query state from
    """
    log.info("Querying session.nextKeys.")
    next_keys = set()
    for account_id, _ in chain_connection.query_map(module="Session",
                                                    storage_function="NextKeys",
                                                    page_size=1000,
                                                    block_hash=block_hash):
        next_keys.add(account_id.value)
    log.debug(f"Found below accounts in Session.nextKeys: {next_keys}")
    log.info("Querying balances.locks")
    locks = {}
    for account_id, lock in chain_connection.query_map(module="Balances",
                                                       storage_function="Locks",
                                                       page_size=1000,
                                                       block_hash=block_hash):
        locks[account_id.value] = lock.value
    log.debug(f"Found below locks: {locks}")
    log.info("Querying staking.bonded")
    bonded = {}
    for account_id, bond in chain_connection.query_map(module="Staking",
                                                       storage_function="Bonded",
                                                       page_size=1000,
                                                       block_hash=block_hash):
        bonded[account_id.value] = bond.value
    log.debug(f"Found below bonds: {bonded}")
    log.info("Querying staking.ledger")
    ledgers = {}
    for account_id, ledger in chain_connection.query_map(module="Staking",
                                                         storage_function="Ledger",
                                                         page_size=1000,
                                                         block_hash=block_hash):
        ledgers[account_id.value] = ledger.serialize()['stash']
    log.debug(f"Found below ledgers: {ledgers}")
    return bonded, ledgers, locks, next_keys


def batch_fix_accounts_consumers_underflow(chain_connection,
                                           input_args,
                                           accounts,
                                           sender_keypair):
    """
    Send Operations.fix_accounts_consumers_underflow call in a batch
    :param chain_connection: WS connection handler
    :param input_args: script input arguments returned from argparse
    :param accounts: list of accounts to fix their consumers counter
    :param sender_keypair: keypair of sender account
    :return: None. Can raise exception in case of SubstrateRequestException thrown
    """
    for (i, account_ids_chunk) in tqdm(iterable=enumerate(chunks(accounts, input_args.fix_consumers_calls_in_batch)),
                                       desc="Accounts checked",
                                       unit="",
                                       file=sys.stdout):
        operations_calls = list(map(lambda account: chain_connection.compose_call(
            call_module='Operations',
            call_function='fix_accounts_consumers_underflow',
            call_params={
                'who': account
            }), account_ids_chunk))
        batch_call = chain_connection.compose_call(
            call_module='Utility',
            call_function='batch_all',
            call_params={
                'calls': operations_calls
            }
        )

        extrinsic = chain_connection.create_signed_extrinsic(call=batch_call, keypair=sender_keypair)
        log.info(f"About to send {len(operations_calls)} Operations.fix_accounts_consumers_underflow, "
                 f"from {sender_keypair.ss58_address} to below accounts: "
                 f"{account_ids_chunk}")

        submit_extrinsic(chain_connection, extrinsic, len(operations_calls), args.dry_run)


def perform_accounts_sanity_checks(chain_connection,
                                   ed,
                                   chain_major_version,
                                   block_hash=None):
    """
    Checks whether all accounts on a chain matches pallet balances invariants
    :param chain_connection: WS connection handler
    :param ed: chain existential deposit
    :param chain_major_version: enum ChainMajorVersion
    :return:None
    """
    invalid_accounts = filter_accounts(chain_connection=chain_connection,
                                       ed=ed,
                                       chain_major_version=chain_major_version,
                                       check_accounts_predicate=lambda x, y, z: not check_account_invariants(x, y, z),
                                       check_accounts_predicate_name="\'incorrect account invariants\'",
                                       block_hash=block_hash)
    if len(invalid_accounts) > 0:
        log.warning(f"Found {len(invalid_accounts)} accounts that do not meet balances invariants!")
        save_accounts_to_json_file("accounts-with-failed-invariants.json", invalid_accounts)
    else:
        log.info(f"All accounts on chain {chain_connection.chain} meet balances invariants.")


if __name__ == "__main__":
    logger.setup_global_logger()
    log = logging.getLogger()
    args = get_args()

    if args.fix_consumers_counter_underflow:
        sender_origin_account_seed = os.getenv('SENDER_ACCOUNT')
        if sender_origin_account_seed is None:
            log.error(f"When specifying "
                      f"--fix-consumers-counter-underflow, env SENDER_ACCOUNT must exists. Exiting.")
            exit(1)
    if args.dry_run:
        log.info(f"Dry-run mode is enabled.")

    chain_ws_connection = substrateinterface.SubstrateInterface(args.ws_url)
    log.info(f"Connected to {chain_ws_connection.name}: {chain_ws_connection.chain} {chain_ws_connection.version}")
    state_block_hash = args.block_hash
    if state_block_hash == '':
        state_block_hash = chain_ws_connection.get_chain_head()
    log.info(f"Script uses block hash {state_block_hash} to query state from.")

    aleph_chain_version = get_aleph_chain_version(chain_ws_connection, state_block_hash)
    log.info(f"Version of an AlephNode chain connected to is {str(aleph_chain_version)}")
    if args.fix_consumers_counter_underflow:
        if aleph_chain_version < AlephChainVersion.VERSION_13_2:
            log.error(
                f"Fixing underflow consumers account can only be done on AlephNode chains with at least "
                f"13.2 version. Exiting.")
            exit(5)

    existential_deposit = chain_ws_connection.get_constant(module_name="Balances",
                                                           constant_name="ExistentialDeposit",
                                                           block_hash=state_block_hash).value
    log.info(f"Existential deposit is {format_balance(chain_ws_connection, existential_deposit)}")

    if args.fix_consumers_counter_underflow:
        log.info(f"This script is going to query all accounts that have underflow of consumers counter, "
                 f"and fix them using runtime Operations.fix_accounts_consumers_underflow extrinsic.")
        accounts_with_consumers_underflow = \
            query_accounts_with_consumers_counter_underflow(chain_connection=chain_ws_connection,
                                                            block_hash=state_block_hash)
        log.info(f"Found {len(accounts_with_consumers_underflow)} accounts with consumers underflow.")
        if len(accounts_with_consumers_underflow) > 0:
            save_accounts_to_json_file("accounts_with_consumers_underflow.json", accounts_with_consumers_underflow)
            code_owners, contract_accounts = contracts.query_contract_and_code_owners_accounts(
                chain_connection=chain_ws_connection,
                block_hash=state_block_hash)
            accounts_with_consumers_underflow_set = set(list(map(lambda x: x[0], accounts_with_consumers_underflow)))
            code_owners_intersection = accounts_with_consumers_underflow_set.intersection(code_owners)
            if len(code_owners_intersection):
                log.warning(
                    f"There are common code owners accounts and consumers underflow accounts: {code_owners_intersection}")
            contract_accounts_intersection = accounts_with_consumers_underflow_set.intersection(contract_accounts)
            if len(contract_accounts_intersection):
                log.warning(
                    f"There are common contract accounts and consumers underflow accounts: {contract_accounts_intersection}")
            sender_origin_account_keypair = substrateinterface.Keypair.create_from_uri(sender_origin_account_seed)
            batch_fix_accounts_consumers_underflow(chain_connection=chain_ws_connection,
                                                   input_args=args,
                                                   sender_keypair=sender_origin_account_keypair,
                                                   accounts=list(accounts_with_consumers_underflow_set))
    if args.check_total_issuance:
        log.info(f"Comparing total issuance aggregated over all accounts with storage value balances.total_issuance")
        total_issuance_from_chain, total_issuance_from_accounts = \
            get_total_issuance_imbalance(chain_connection=chain_ws_connection,
                                         block_hash=state_block_hash)
        log_total_issuance_imbalance(chain_connection=chain_ws_connection,
                                     total_issuance_from_chain=total_issuance_from_chain,
                                     total_issuance_from_accounts=total_issuance_from_accounts,
                                     block_hash=state_block_hash)
        delta = total_issuance_from_chain - total_issuance_from_accounts
        if delta != 0:
            if not args.start_range_block_hash:
                log.error(f"--start-range-block-hash must be set when --check-total-issuance is set "
                          f"to perform further actions. Exiting.")
                sys.exit(2)
            log.warning(f"Total issuance retrieved from the chain storage is different than aggregated sum over"
                        f" all accounts. Finding first block when it happened.")
            first_block_hash_imbalance = find_block_hash_with_imbalance(chain_connection=chain_ws_connection,
                                                                        start_block_hash=args.start_range_block_hash,
                                                                        end_block_hash=state_block_hash)
            log.info(f"The first block where it happened is {first_block_hash_imbalance}")

    log.info(f"Performing pallet balances sanity checks.")
    perform_accounts_sanity_checks(chain_connection=chain_ws_connection,
                                   ed=existential_deposit,
                                   chain_major_version=aleph_chain_version,
                                   block_hash=state_block_hash)
    log.info(f"DONE")
