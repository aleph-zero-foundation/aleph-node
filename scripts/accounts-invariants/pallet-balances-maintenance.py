#!/bin/python3
import copy
import datetime
import json
import pathlib

import substrateinterface
import argparse
import logging
import enum
import os

from substrateinterface.storage import StorageKey


class ChainMajorVersion(enum.Enum):
    PRE_12_MAJOR_VERSION = 65,
    AT_LEAST_12_MAJOR_VERSION = 68,
    AT_LEAST_13_1_VERSION = 70,

    @classmethod
    def from_spec_version(cls, spec_version):
        if spec_version <= 65:
            return cls(ChainMajorVersion.PRE_12_MAJOR_VERSION)
        elif 68 <= spec_version < 70:
            return cls(ChainMajorVersion.AT_LEAST_12_MAJOR_VERSION)
        elif spec_version >= 70:
            return cls(ChainMajorVersion.AT_LEAST_13_1_VERSION)


def get_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="""
Script for maintenance operations on AlephNode chain with regards to pallet balances.

It has following functionality: 
* workaround for bug https://github.com/paritytech/polkadot-sdk/pull/2700/files, that make sure all 
  accounts have at least ED as their free balance,
* programmatic support for sending Balances.UpgradeAccounts ext for all accounts,
* checking pallet balances and account reference counters invariants.

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
    parser.add_argument('--transfer-calls-in-batch',
                        type=int,
                        default=64,
                        help='How many transfer calls to perform in a single batch transaction. Default: 64')
    parser.add_argument('--upgrade-accounts-in-batch',
                        type=int,
                        default=128,
                        help='How many accounts to upgrade in a single transaction. Default: 128')
    parser.add_argument('--double-providers-accounts-to-fix',
                        type=int,
                        default=10,
                        help='How many accounts to fix in one script run session.'
                             ' Default: 10')
    parser.add_argument('--fix-consumers-calls-in-batch',
                        type=int,
                        default=10,
                        help='How many consumers underflow accounts to fix in one script run session.'
                             ' Default: 10')
    group = parser.add_mutually_exclusive_group()
    group.add_argument('--fix-free-balance',
                       action='store_true',
                       help='Specify this switch if script should find all accounts '
                            'that have free < ED and send transfers so that free >= ED. '
                            'It requires AlephNode < 12.X version and SENDER_ACCOUNT env to be set with '
                            'a mnemonic phrase of sender account that has funds for transfers and fees. '
                            'dust-accounts.json file is saved with all such accounts.'
                            'Must be used exclusively with --upgrade-accounts or --fix-double-providers-count.'
                            'Default: False')
    group.add_argument('--upgrade-accounts',
                       action='store_true',
                       help='Specify this switch if script should send Balances.UpgradeAccounts '
                            'for all accounts on a chain. It requires at least AlephNode 12.X version '
                            'and SENDER_ACCOUNT env to be set with a mnemonic phrase of sender account that has funds '
                            'for transfers and fees.'
                            'Must be used exclusively with --fix-free-balance or --fix-double-providers-count.'
                            'Default: False')
    group.add_argument('--fix-double-providers-count',
                       action='store_true',
                       help='Specify this switch if script should query all accounts that have providers == 2'
                            'and decrease this counter by one using System.SetStorage call. '
                            'It requires at least AlephNode 12.X version '
                            'and SENDER_ACCOUNT env to be set with a sudo mnemonic phrase. '
                            'Must be used exclusively with --fix-free-balance or --upgrade-accounts.'
                            'Default: False')
    group.add_argument('--fix-consumers-counter-underflow',
                       action='store_true',
                       help='Specify this switch if script should query all accounts that have underflow consumers '
                            'counter, and fix consumers counter by issuing batch calls of '
                            'Operations.fix_accounts_consumers_underflow call.'
                            'It requires at least AlephNode 13.1 version '
                            'Default: False')
    return parser.parse_args()


def get_chain_major_version(chain_connection):
    """
    Retrieves spec_version from chain and returns an enum whether this is pre 12 version or at least 12 version
    :param chain_connection: WS handler
    :return: ChainMajorVersion
    """
    runtime_version = chain_connection.get_block_runtime_version(None)
    spec_version = runtime_version['specVersion']
    major_version = ChainMajorVersion.from_spec_version(spec_version)
    return major_version


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

    if chain_major_version == ChainMajorVersion.PRE_12_MAJOR_VERSION:
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


def filter_accounts(chain_connection,
                    ed,
                    chain_major_version,
                    check_accounts_predicate,
                    check_accounts_predicate_name=""):
    """
    Filters out all chain accounts by given predicate.
    :param chain_connection: WS handler
    :param ed: existential deposit
    :param chain_major_version: enum ChainMajorVersion
    :param check_accounts_predicate: a function that takes three arguments predicate(account, chain_major_version, ed)
    :param check_accounts_predicate_name: name of the predicate, used for logging reasons only
    :return: a list which has those chain accounts which returns True on check_accounts_predicate
    """
    accounts_that_do_meet_predicate = []
    # query_map reads state from the **single** block, if block_hash is not None this is top of the chain
    account_query = chain_connection.query_map(module='System',
                                               storage_function='Account',
                                               page_size=1000)
    total_accounts_count = 0

    for (i, (account_id, info)) in enumerate(account_query):
        total_accounts_count += 1
        if check_accounts_predicate(info, chain_major_version, ed):
            accounts_that_do_meet_predicate.append([account_id.value, info.serialize()])
        if i % 5000 == 0 and i > 0:
            log.info(f"Checked {i} accounts")

    log.info(
        f"Total accounts that match given predicate {check_accounts_predicate_name} is {len(accounts_that_do_meet_predicate)}")
    log.info(f"Total accounts checked: {total_accounts_count}")
    return accounts_that_do_meet_predicate


def check_if_account_would_be_dust_in_12_version(account, chain_major_version, ed):
    """
    This predicate checks if a valid account in pre-12 version will be invalid in version 12.

    :param account: AccountInfo struct (element of System.Accounts StorageMap)
    :param chain_major_version: Must be < 12
    :param ed: existential deposit
    :return: True if account free balance < ED, False otherwise
    """

    assert chain_major_version == ChainMajorVersion.PRE_12_MAJOR_VERSION, \
        "Chain major version must be less than 12!"
    assert check_account_invariants(account, chain_major_version, ed), \
        f"Account {account} does not meet pre-12 version invariants!"

    free = account['data']['free'].value

    return free < ed


def find_dust_accounts(chain_connection, ed, chain_major_version):
    """
    This function finds all accounts that are valid in 11 version, but not on 12 version
    """
    assert chain_major_version == ChainMajorVersion.PRE_12_MAJOR_VERSION, \
        "Chain major version must be less than 12!"
    return filter_accounts(chain_connection=chain_connection,
                           ed=ed,
                           chain_major_version=chain_major_version,
                           check_accounts_predicate=check_if_account_would_be_dust_in_12_version,
                           check_accounts_predicate_name="\'account valid in pre-12 version but not in 12 version\'")


def format_balance(chain_connection, amount):
    """
    Helper method to display underlying U128 Balance type in human-readable form
    :param chain_connection: WS connection handler (for retrieving token symbol metadata)
    :param amount: ammount to be formatted
    :return: balance in human-readable form
    """
    decimals = chain_connection.token_decimals or 12
    amount = format(amount / 10 ** decimals)
    token = chain_connection.token_symbol
    return f"{amount} {token}"


def batch_transfer(chain_connection,
                   input_args,
                   accounts,
                   amount,
                   sender_keypair):
    """
    Send Balance.Transfer calls in a batch
    :param chain_connection: WS connection handler
    :param input_args: script input arguments returned from argparse
    :param accounts: transfer beneficents
    :param amount: amount to be transferred
    :param sender_keypair: keypair of sender account
    :return: None. Can raise exception in case of SubstrateRequestException thrown
    """
    for (i, account_ids_chunk) in enumerate(chunks(accounts, input_args.transfer_calls_in_batch)):
        balance_calls = list(map(lambda account: chain_connection.compose_call(
            call_module='Balances',
            call_function='transfer',
            call_params={
                'dest': account,
                'value': amount,
            }), account_ids_chunk))
        batch_call = chain_connection.compose_call(
            call_module='Utility',
            call_function='batch',
            call_params={
                'calls': balance_calls
            }
        )

        extrinsic = chain_connection.create_signed_extrinsic(call=batch_call, keypair=sender_keypair)
        log.info(f"About to send {len(balance_calls)} transfers, each with {format_balance(chain_connection, amount)} "
                 f"from {sender_keypair.ss58_address} to below accounts: "
                 f"{account_ids_chunk}")

        submit_extrinsic(chain_connection, extrinsic, len(balance_calls), args.dry_run)


def submit_extrinsic(chain_connection,
                     extrinsic,
                     expected_number_of_events,
                     dry_run):
    """
    Submit a signed extrinsic
    :param chain_connection: WS connection handler
    :param extrinsic: an ext to be sent
    :param expected_number_of_events: how many events caller expects to be emitted from chain
    :param dry_run: boolean whether to actually send ext or not
    :return: Hash of block extrinsic was included or None for dry-run.
             Can raise exception in case of SubstrateRequestException thrown when sending ext.
    """
    try:
        log.debug(f"Extrinsic to be sent: {extrinsic}")
        if not dry_run:
            receipt = chain_connection.submit_extrinsic(extrinsic, wait_for_inclusion=True)
            log.info(f"Extrinsic included in block {receipt.block_hash}: "
                     f"Paid {format_balance(chain_connection, receipt.total_fee_amount)}")
            if receipt.is_success:
                log.debug("Extrinsic success.")
                if len(receipt.triggered_events) < expected_number_of_events:
                    log.debug(
                        f"Emitted fewer events than expected: "
                        f"{len(receipt.triggered_events)} < {expected_number_of_events}")
                log.debug(f"Emitted events:")
                for event in receipt.triggered_events:
                    log.debug(f'* {event.value}')
            else:
                log.warning(f"Extrinsic failed with following message: {receipt.error_message}")
            return receipt.block_hash
        else:
            log.info(f"Not sending extrinsic, --dry-run is enabled.")
    except substrateinterface.exceptions.SubstrateRequestException as e:
        log.warning(f"Failed to submit extrinsic: {e}")
        raise e


def upgrade_accounts(chain_connection,
                     input_args,
                     ed,
                     chain_major_version,
                     sender_keypair):
    """
    Prepare and send Balances.UpgradeAccounts call for all accounts on a chain
    :param chain_connection: WS connection handler
    :param input_args: script input arguments returned from argparse
    :param ed: chain existential deposit
    :param chain_major_version: enum ChainMajorVersion
    :param sender_keypair: keypair of sender account
    :return: None. Can raise exception in case of SubstrateRequestException thrown
    """
    log.info("Querying all accounts.")
    all_accounts_on_chain = list(map(lambda x: x[0], get_all_accounts(chain_connection)))

    for (i, account_ids_chunk) in enumerate(chunks(all_accounts_on_chain, input_args.upgrade_accounts_in_batch)):
        upgrade_accounts_call = chain_connection.compose_call(
            call_module='Balances',
            call_function='upgrade_accounts',
            call_params={
                'who': account_ids_chunk,
            }
        )

        extrinsic = chain_connection.create_signed_extrinsic(call=upgrade_accounts_call, keypair=sender_keypair)
        log.info(
            f"About to upgrade {len(account_ids_chunk)} accounts, each with "
            f"{format_balance(chain_connection, existential_deposit)}")

        submit_extrinsic(chain_connection, extrinsic, len(account_ids_chunk), args.dry_run)


def check_if_account_has_double_providers(account, chain_major_version, ed):
    """
    This predicate checks if an account's providers counter is equal to 2
    :param account: AccountInfo struct (element of System.Accounts StorageMap)
    :param chain_major_version: Must be >= 12
    :param ed: existential deposit, not used
    :return: True if providers == 2, False otherwise
    """

    assert chain_major_version == ChainMajorVersion.AT_LEAST_12_MAJOR_VERSION, \
        "Chain major version must be at least 12!"

    providers = account['providers'].value

    return providers == 2


def state_sanity_check(chain_connection,
                       account_id,
                       set_storage_block_hash):
    """
    Makes sure AccountInfo data changed only in terms of providers counter by comparing state between block that
    setStorage landed vs state of that block parent.
    :param chain_connection WS connection handler
    :param account_id: An account to check
    :param set_storage_block_hash: a hash of a block that contains setStorage
    :return: None.
    """
    set_storage_block_number = chain_connection.get_block_number(set_storage_block_hash)
    parent_block_number = set_storage_block_number - 1
    parent_block_hash = chain_connection.get_block_hash(parent_block_number)

    log.info(f"Comparing AccountInfo for account {account_id} "
             f"between blocks #{parent_block_number} and #{set_storage_block_number}")
    parent_account_info_state = chain_connection.query(module='System',
                                                       storage_function='Account',
                                                       params=[account_id],
                                                       block_hash=parent_block_hash).value
    log.debug(f"Account state in the parent block: {parent_account_info_state}")
    set_storage_info_state = chain_connection.query(module='System',
                                                    storage_function='Account',
                                                    params=[account_id],
                                                    block_hash=set_storage_block_hash).value
    log.debug(f"Account state in the setStorage block: {set_storage_info_state}")
    assert parent_account_info_state['providers'] == 2, f"Providers before fix for account {account_id} is not 2!"
    assert set_storage_info_state['providers'] == 1, f"Providers after fix for account {account_id} is not 1!"
    changed_state_for_sake_of_assert = copy.deepcopy(parent_account_info_state)
    changed_state_for_sake_of_assert['providers'] = 1
    assert changed_state_for_sake_of_assert == set_storage_info_state, \
        f"Parent account info state is different from set storage state with more than providers counter! " \
        f"Parent state: {parent_account_info_state} " \
        f"Set storage state: {set_storage_info_state}" \
        f"Set storage block number {set_storage_block_number}"


def get_system_account_metadata_scale_codec_type(chain_connection):
    """
     Retrieves System.Account metadata decoder type
    :param chain_connection WS connection handler
    :return: string representation of System.Account decoder type
    """
    system_account_storage_function = \
        next(filter(
            lambda storage_function: storage_function['storage_name'] == 'Account' and
                                     storage_function['module_id'] == 'System',
            chain_connection.get_metadata_storage_functions()))
    internal_metadata_scale_codec_type = system_account_storage_function['type_value']
    return internal_metadata_scale_codec_type


def fix_double_providers_count_for_account(chain_connection,
                                           account_id,
                                           sudo_sender_keypair,
                                           internal_system_account_scale_codec_type,
                                           input_args):
    """
    Fixes double providers counter for a given account id.
    :param chain_connection: WS connection handler
    :param account_id: Account id to which we should fix providers counter
    :param sudo_sender_keypair Mnemonic phrase of sudo
    :param internal_system_account_scale_codec_type Internal metadata SCALE decoder/encoder type for System.Account
           entry
    """
    log.info(f"Fixing double providers counter for account {account_id}")
    log.info(f"Querying state for account {account_id}")
    result = chain_connection.query('System', 'Account', [account_id])
    log.debug(f"Returned value: {result.value}")
    account_id_and_account_info_data = [(account_id, result.value)]
    raw_key_values = get_raw_key_values(chain_connection,
                                        account_id_and_account_info_data,
                                        internal_system_account_scale_codec_type)

    set_storage_call = chain_connection.compose_call(
        call_module='System',
        call_function='set_storage',
        call_params={
            'items': raw_key_values,
        })
    # ref time is set to 400ms to make sure this is the only tx that ends up in a block
    # 359 875 586 000 is a maximal weight (found empirically) that sudo_unchecked_weight is able to consume
    max_weight = 359875586000
    sudo_unchecked_weight_call = chain_connection.compose_call(
        call_module='Sudo',
        call_function='sudo_unchecked_weight',
        call_params={
            'call': set_storage_call,
            'weight': {
                'proof_size': 0,
                'ref_time': max_weight,
            },
        }
    )

    # add a small tip to make sure this will be the first transaction in the block
    token_mili_unit = 1000000000
    extrinsic = chain_connection.create_signed_extrinsic(call=sudo_unchecked_weight_call,
                                                         keypair=sudo_sender_keypair,
                                                         tip=token_mili_unit)
    block_hash = submit_extrinsic(chain_connection, extrinsic, 1, input_args.dry_run)
    if not input_args.dry_run:
        state_sanity_check(chain_connection,
                           account_id,
                           block_hash)


def fix_double_providers_count(chain_connection,
                               input_args,
                               chain_major_version,
                               sudo_sender_keypair):
    """
    Queries those accounts using System.Account map which have providers == 2.
    For each such account, performs System.SetStorage with the same data but providers set to 1.
    Must be run on AlephNode chain with at least 12 version.
    :param chain_connection: WS connection handler
    :param input_args: script input arguments returned from argparse
    :param chain_major_version: enum ChainMajorVersion
    :param sudo_sender_keypair: sudo keypair of sender account
    :return: None. Can raise exception in case of SubstrateRequestException thrown
    """
    log.info("Querying all accounts that have double provider counter.")
    double_providers_accounts = filter_accounts(chain_connection=chain_connection,
                                                ed=None,
                                                chain_major_version=chain_major_version,
                                                check_accounts_predicate=check_if_account_has_double_providers,
                                                check_accounts_predicate_name="\'double provider count\'")
    log.info(f"Found {len(double_providers_accounts)} accounts with double provider counter.")
    if len(double_providers_accounts) > 0:
        save_accounts_to_json_file("double-providers-accounts.json", double_providers_accounts)
    log.info(f"Will fix at most first {input_args.double_providers_accounts_to_fix} accounts.")
    internal_system_account_scale_codec_type = get_system_account_metadata_scale_codec_type(chain_connection)
    for account_id, _ in double_providers_accounts[:input_args.double_providers_accounts_to_fix]:
        fix_double_providers_count_for_account(chain_connection,
                                               account_id,
                                               sudo_sender_keypair,
                                               internal_system_account_scale_codec_type,
                                               input_args)


def get_raw_key_values(chain_connection, account_id_and_data_chunk, internal_system_account_scale_codec_type):
    """
    Prepares input arguments for System.setStorage calls wth fixed providers counter
    :param chain_connection: WS connection handler. Used for passing metadata when creating storage keys, which
                             is a valid assumption that it's not going to change during this script execution
    :param account_id_and_data_chunk: A list of tuples (account_id, decoded_account_info)
    :param internal_system_account_scale_codec_type Internal metadata SCALE decoder/encoder type for System.Account
           entry
    :return: List of tuples (system_account_storage_key_hexstring, account_info_raw_value_hexstring) ready to be sent
             to System.setStorage call
    """
    account_ids_chunk = list(
        map(lambda account_id_and_data: account_id_and_data[0], account_id_and_data_chunk))
    system_account_storage_keys_hexstrings = list(map(
        lambda account_id:
        StorageKey.create_from_storage_function(pallet="System",
                                                storage_function="Account",
                                                params=[account_id],
                                                runtime_config=chain_connection.runtime_config,
                                                metadata=chain_connection.metadata).to_hex(),
        account_ids_chunk))
    account_info_chunk = list(
        map(lambda account_id_and_data: account_id_and_data[1], account_id_and_data_chunk))
    raw_hexstring_values = list(map(lambda account_info: set_providers_counter_to_one(chain_connection,
                                                                                      account_info,
                                                                                      internal_system_account_scale_codec_type),
                                    account_info_chunk))
    raw_key_values = list(zip(system_account_storage_keys_hexstrings, raw_hexstring_values))
    log.info(f"Prepared {len(raw_key_values)} raw key value pairs.")
    log.debug(f"{raw_key_values}")
    return raw_key_values


def assert_same_data_except_providers_counter(account_data_hexstring,
                                              account_data_with_fixed_providers_counter_hexstring):
    """
    Function makes sure previous and fixed account data is different only in providers counter
    :param account_data_hexstring: Hexstring (raw value) representation of original AccountData
    :param account_data_with_fixed_providers_counter_hexstring: Hexstring representation (raw value) of AccountData with
           fixed providers counter
    :return: None, but raises AssertionError in case data is different not only in providers counter
    """
    # example hexstring of AccountInfo is
    # 0x00000000000000000100000000000000f4010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000080
    # difference can be only on byte 20th, which is LSB of providers counter that must be equal to 1 in fixed data
    assert account_data_hexstring[:19] == account_data_with_fixed_providers_counter_hexstring[:19], \
        f"First 20 bytes of original and fixed AccountInfo must be equal"
    assert account_data_with_fixed_providers_counter_hexstring[19] == '1', \
        f"Providers counter of fixed AccountInfo must be 1"
    assert account_data_hexstring[20:] == account_data_with_fixed_providers_counter_hexstring[20:], \
        f"Last but 21 bytes of original and fixed AccountInfo must be equal"


def set_providers_counter_to_one(chain_connection, account_info, internal_system_account_scale_codec_type):
    """
    Method sets provider counter for a System.Account to 1 using System.SetStorage call. Since we must replace whole
    System.Account value, which contains also other account counters as well as balances data for the account, this
    solution is prone to a race condition in which we this data is altered meanwhile we issue set_storage. Practically,
    this can happen either that here is a transaction that ends up in the same block as set_storage or just before,
    causing a (write) race condition. In order to prevent that one needs to read state of parent of the block that
    contains setStorge transaction and make sure only difference in state is providers counter.

    This function encodes original AccountInfo with fixed providers count (set to 1). It also asserts
    original and fixed AccountInfo, encoded as hexstrings, is different only on the providers counter.

    :param chain_connection: WS connection handler. Uses for passing metadata when creating storage keys, which
                             is a valid assumption that it's not going to change during this script execution
    :param account_info: decoded AccountInfo that has double providers counter
    :param internal_system_account_scale_codec_type Internal metadata SCALE decoder/encoder type for System.Account
           entry
    :return: Raw storage value hexstring representation of AccountInfo with providers counter set to 1
    """
    fixed_account_data = copy.deepcopy(account_info)
    fixed_account_data['providers'] = 1
    scale_object = chain_connection.runtime_config.create_scale_object(
        type_string=internal_system_account_scale_codec_type, metadata=chain_connection.metadata
    )
    account_data_with_fixed_providers_counter = scale_object.encode(fixed_account_data)
    fixed_account_info_hexstring = account_data_with_fixed_providers_counter.to_hex()
    original_encoded_data_hexstring = scale_object.encode(account_info).to_hex()
    assert_same_data_except_providers_counter(original_encoded_data_hexstring,
                                              fixed_account_info_hexstring)
    return fixed_account_info_hexstring


def query_contract_and_code_owners_accounts(chain_connection):
    """
    Returns contract accounts and code owners.
    """
    code_owners = set()
    contract_accounts = set()

    log.info(f"Querying code owners.")
    code_info_of_query = chain_connection.query_map('Contracts', 'CodeInfoOf', page_size=1000)

    for (i, (account_id, info)) in enumerate(code_info_of_query):
        code_owners.add(info.serialize()['owner'])
        if i % 5000 == 0 and i > 0:
            log.info(f"Checked {i} code owners")
    log.info(f"Total code owners is {len(code_owners)}")
    log.debug(f"Code owners: {code_owners}")

    log.info(f"Querying contract accounts.")
    contract_info_of_query = chain_connection.query_map('Contracts', 'ContractInfoOf', page_size=1000)
    for (i, (account_id, info)) in enumerate(contract_info_of_query):
        contract_accounts.add(account_id.value)
        if i % 5000 == 0 and i > 0:
            log.info(f"Checked {i} contracts")
    log.info(f"Total contracts count is {len(contract_accounts)}")
    log.debug(f"Contract accounts: {contract_accounts}")

    return code_owners, contract_accounts


def no_consumers_zero_reserved(account_id, account_info):
    """
    Checks if an account has zero consumers, but non-zero reserved amount.
    """
    consumers = account_info['consumers']
    reserved = account_info['data']['reserved']
    if consumers == 0 and reserved > 0:
        log.debug(f"Found an account that has zero consumers and non-zero reserved: {account_id}")
        return True
    return False


def staker_has_consumers_underflow(account_id, consumers, locks, next_keys):
    """
    Checks if an account is a staker or vester that has underflow consumers counter.
    """
    vesting_lock_encoded_id = "0x76657374696e6720"
    staking_lock_encoded_id = "0x7374616b696e6720"
    if account_id in locks and len(locks[account_id]) > 0:
        log.debug(f"Account {account_id} has following locks: {locks[account_id]}")
        vesting_lock = next(filter(lambda lock: lock['id'] == vesting_lock_encoded_id, locks[account_id]), None)
        if vesting_lock is not None and consumers == 1:
            log.debug(f"Found vesting lock: {vesting_lock}")
            log.debug(f"Found an account that has one consumer and vesting lock: {account_id}")
            return True
        staking_lock = next(filter(lambda lock: lock['id'] == staking_lock_encoded_id, locks[account_id]), None)
        if staking_lock is not None:
            log.debug(f"Found staking lock: {staking_lock}")
            if consumers == 2:
                log.debug(f"Found an account that has two consumers and staking lock: {account_id}")
                return True
            if consumers == 3 and account_id in next_keys:
                log.debug(f"Found an account that has three consumers, staking lock, and next session key: "
                          f"{account_id}")
                return True
    return False


def has_account_consumer_underflow(account_id_and_info, locks, next_keys):
    """
    Returns True if an account has consumers underflow
    """
    account_id, account_info = account_id_and_info
    consumers = account_info['consumers']
    return no_consumers_zero_reserved(account_id, account_info) or \
        staker_has_consumers_underflow(account_id, consumers,locks, next_keys)


def query_accounts_with_consumers_counter_underflow(chain_connection):
    """
    Queries all accounts that have an underflow of consumers counter, which is either of those account categories:
    * `consumers`  == 0, `reserved`  > 0
    * `consumers`  == 1, `balances.Locks` contain an entry with `id`  == `vesting`
    * `consumers`  == 2, `balances.Locks` contain an entry with `id`  == `staking`
    * `consumers`  == 3, `balances.Locks` contain entries with `id`  == `staking` and accountId is in `session.nextKeys`
    """
    log.info("Querying session.nextKeys.")
    next_keys = set()
    for account_id, _ in chain_connection.query_map("Session", "NextKeys", page_size=1000):
        next_keys.add(account_id.value)
    log.debug(f"Found below accounts in Session.nextKeys: {next_keys}")

    log.info("Querying balances.locks.")
    locks = {}
    for account_id, lock in chain_connection.query_map("Balances", "Locks", page_size=1000):
        locks[account_id.value] = lock.value
    log.debug(f"Found below locks: {locks}")

    log.info("Querying all accounts and filtering by consumers underflow predicate.")
    return [account_id_and_info for account_id_and_info in get_all_accounts(chain_connection) if
            has_account_consumer_underflow(account_id_and_info, locks, next_keys)]


def get_all_accounts(chain_connection):
    return filter_accounts(chain_connection,
                           None,
                           None,
                           lambda x, y, z: True,
                           "\'all accounts\'")


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
    for (i, account_ids_chunk) in enumerate(chunks(accounts, input_args.fix_consumers_calls_in_batch)):
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


def chunks(list_of_elements, n):
    """
    Lazily split 'list_of_elements' into 'n'-sized chunks.
    """
    for i in range(0, len(list_of_elements), n):
        yield list_of_elements[i:i + n]


def perform_account_sanity_checks(chain_connection,
                                  ed,
                                  chain_major_version):
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
                                       check_accounts_predicate_name="\'incorrect account invariants\'")
    if len(invalid_accounts) > 0:
        log.warning(f"Found {len(invalid_accounts)} accounts that do not meet balances invariants!")
        save_accounts_to_json_file("accounts-with-failed-invariants.json", invalid_accounts)
    else:
        log.info(f"All accounts on chain {chain_connection.chain} meet balances invariants.")


def save_accounts_to_json_file(json_file_name, accounts):
    with open(json_file_name, 'w') as f:
        json.dump(accounts, f)
        log.info(f"Wrote file '{json_file_name}'")


def get_global_logger(input_args):
    log_formatter = logging.Formatter("%(asctime)s [%(levelname)s] %(message)s")
    root_logger = logging.getLogger()
    root_logger.setLevel(input_args.log_level.upper())

    time_now = datetime.datetime.now().strftime("%d-%m-%Y_%H:%M:%S")
    script_name_without_extension = pathlib.Path(__file__).stem
    file_handler = logging.FileHandler(f"{script_name_without_extension}-{time_now}.log")
    file_handler.setFormatter(log_formatter)
    file_handler.setLevel(logging.DEBUG)
    root_logger.addHandler(file_handler)

    console_handler = logging.StreamHandler()
    console_handler.setFormatter(log_formatter)
    console_handler.setLevel(logging.INFO)
    root_logger.addHandler(console_handler)

    return logging


if __name__ == "__main__":
    args = get_args()
    log = get_global_logger(args)

    if args.fix_free_balance or args.upgrade_accounts or args.fix_double_providers_count \
            or args.fix_consumers_counter_underflow:
        sender_origin_account_seed = os.getenv('SENDER_ACCOUNT')
        if sender_origin_account_seed is None:
            log.error(f"When specifying --fix-free-balance or --upgrade-accounts or --fix-double-providers-count or "
                      f"--fix-consumers-counter-underflow, env SENDER_ACCOUNT must exists. Exiting.")
            exit(1)
    if args.dry_run:
        log.info(f"Dry-run mode is enabled.")

    chain_ws_connection = substrateinterface.SubstrateInterface(args.ws_url)
    log.info(f"Connected to {chain_ws_connection.name}: {chain_ws_connection.chain} {chain_ws_connection.version}")

    chain_major_version = get_chain_major_version(chain_ws_connection)
    log.info(f"Major version of chain connected to is {chain_major_version}")
    if args.fix_free_balance:
        if chain_major_version is not ChainMajorVersion.PRE_12_MAJOR_VERSION:
            log.error(f"--fix-free-balance can be used only on chains with pre-12 version. Exiting.")
            exit(2)
    if args.upgrade_accounts:
        if chain_major_version is not ChainMajorVersion.AT_LEAST_12_MAJOR_VERSION:
            log.error(f"--upgrade-accounts can be used only on chains with at least 12 version. Exiting.")
            exit(3)
    if args.fix_double_providers_count:
        if chain_major_version is not ChainMajorVersion.AT_LEAST_12_MAJOR_VERSION:
            log.error(f"--fix-double-providers-count can be used only on chains with at least 12 version. Exiting.")
            exit(4)
    if args.fix_consumers_counter_underflow:
        if chain_major_version is not ChainMajorVersion.AT_LEAST_13_1_VERSION:
            log.error(f"--query-consumers-counter-underflow can be used only on chains with at least 13.1 version. "
                      f"Exiting.")
            exit(5)

    existential_deposit = chain_ws_connection.get_constant("Balances", "ExistentialDeposit").value
    log.info(f"Existential deposit is {format_balance(chain_ws_connection, existential_deposit)}")

    if args.fix_free_balance:
        sender_origin_account_keypair = substrateinterface.Keypair.create_from_uri(sender_origin_account_seed)
        log.info(f"Using following account for transfers: {sender_origin_account_keypair.ss58_address}")
        log.info(f"Will send at most {args.transfer_calls_in_batch} transfers in a batch.")
        log.info(f"Looking for accounts that would be dust in 12 version.")
        dust_accounts_in_12_version = find_dust_accounts(chain_ws_connection, existential_deposit, chain_major_version)
        if len(dust_accounts_in_12_version):
            log.info(f"Found {len(dust_accounts_in_12_version)} accounts that will be invalid in 12 version.")
            save_accounts_to_json_file("dust-accounts.json", dust_accounts_in_12_version)
            log.info("Adjusting balances by sending transfers.")
            batch_transfer(chain_connection=chain_ws_connection,
                           input_args=args,
                           accounts=list(map(lambda x: x[0], dust_accounts_in_12_version)),
                           amount=existential_deposit,
                           sender_keypair=sender_origin_account_keypair)
            log.info(f"Transfers done.")
        else:
            log.info(f"No dust accounts found, skipping transfers.")
    if args.upgrade_accounts:
        sender_origin_account_keypair = substrateinterface.Keypair.create_from_uri(sender_origin_account_seed)
        log.info(f"Using following account for upgrade_accounts: {sender_origin_account_keypair.ss58_address}")
        log.info(f"Will upgrade at most {args.upgrade_accounts_in_batch} accounts in a batch.")
        upgrade_accounts(chain_connection=chain_ws_connection,
                         input_args=args,
                         ed=existential_deposit,
                         chain_major_version=chain_major_version,
                         sender_keypair=sender_origin_account_keypair)
        log.info("Upgrade accounts done.")
    if args.fix_double_providers_count:
        sudo_account_keypair = substrateinterface.Keypair.create_from_uri(sender_origin_account_seed)
        log.info(f"This script is going to query all accounts that have providers == 2 and decrease this counter "
                 f"by one using System.SetStorage extrinsic, which requires sudo.")
        log.info(f"Using the following account for System.SetStorage calls: {sudo_account_keypair.ss58_address}")
        fix_double_providers_count(chain_connection=chain_ws_connection,
                                   input_args=args,
                                   chain_major_version=chain_major_version,
                                   sudo_sender_keypair=sudo_account_keypair)
    if args.fix_consumers_counter_underflow:
        log.info(f"This script is going to query all accounts that have underflow of consumers counter, "
                 f"and fix them using runtime Operations.fix_accounts_consumers_underflow extrinsic.")
        accounts_with_consumers_underflow = \
            query_accounts_with_consumers_counter_underflow(chain_connection=chain_ws_connection)
        log.info(f"Found {len(accounts_with_consumers_underflow)} accounts with consumers underflow.")
        if len(accounts_with_consumers_underflow) > 0:
            save_accounts_to_json_file("accounts_with_consumers_underflow.json", accounts_with_consumers_underflow)
            code_owners, contract_accounts = query_contract_and_code_owners_accounts(
                chain_connection=chain_ws_connection)
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

    log.info(f"Performing pallet balances sanity checks.")
    perform_account_sanity_checks(chain_connection=chain_ws_connection,
                                  ed=existential_deposit,
                                  chain_major_version=chain_major_version)
    log.info(f"DONE")
