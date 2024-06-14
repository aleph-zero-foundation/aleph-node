#!/bin/python3

from chain_operations import *
from consumers_counter import *
from aleph_chain_version import *
from contracts import *
from total_issuance import *
from account_invariants import *

import substrateinterface
import argparse
import os
import logger
import logging


def get_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="""
Script for maintenance operations on AlephNode chain with regards to pallet balances.

It has following functionality: 
* checking pallet balances and account reference counters invariants.
* calling pallet operations for maintenance actions
* checking total issuance 

By default, it connects to a AlephZero Testnet and performs read-only state logic checks.
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
    parser.add_argument('--fix-accounts-in-batch',
                        type=int,
                        default=10,
                        help='How many accounts to fix in one script run session.'
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
    parser.add_argument('--fix-consumers-counter',
                        action='store_true',
                        help='Specify this switch if script should fix any consumers counter discrepancy found.'
                             'Operations.fix_accounts_consumers_counter call is used, that takes up to '
                             '--fix-accounts-in-batch accounts. It requires AlephNode >= 14.0'
                             'Default: False')
    return parser.parse_args()


if __name__ == "__main__":
    logger.setup_global_logger()
    log = logging.getLogger()
    args = get_args()

    if args.fix_consumers_counter:
        sender_origin_account_seed = os.getenv('SENDER_ACCOUNT')
        if sender_origin_account_seed is None:
            log.error(f"When specifying "
                      f"--fix-consumers-counter, env SENDER_ACCOUNT must exists. Exiting.")
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
    if args.fix_consumers_counter:
        if aleph_chain_version < AlephChainVersion.VERSION_14_X:
            log.error(
                f"Fixing consumers counter can only be done on AlephNode chains with at least "
                f"14.0 version. Exiting.")
            exit(5)

    bonded, ledger, locks, next_keys = query_staking_data(chain_ws_connection, state_block_hash)
    code_owners, contract_accounts = query_contract_and_code_owners_accounts(chain_ws_connection, state_block_hash)
    all_accounts = get_all_accounts(chain_ws_connection, state_block_hash)

    if args.fix_consumers_counter:
        log.info(f"Fixing all accounts that have invalid consumers counter")
        accounts_with_invalid_consumers_counter = [account_id_and_info for account_id_and_info in
                                                   all_accounts if
                                                   not check_consumers_counter(aleph_chain_version,
                                                                               account_id_and_info[0],
                                                                               account_id_and_info[1],
                                                                               locks,
                                                                               bonded,
                                                                               ledger,
                                                                               next_keys,
                                                                               contract_accounts)]

        log.info(f"Found {len(accounts_with_invalid_consumers_counter)} accounts with an invalid consumers counter.")
        if len(accounts_with_invalid_consumers_counter) > 0:
            save_accounts_to_json_file("accounts_with_invalid_consumers_counter.json",
                                       accounts_with_invalid_consumers_counter)
            sender_origin_account_keypair = substrateinterface.Keypair.create_from_uri(sender_origin_account_seed)
            batch_fix_accounts_consumers_counter(chain_connection=chain_ws_connection,
                                                 input_args=args,
                                                 sender_keypair=sender_origin_account_keypair,
                                                 accounts=list(map(lambda accoount_if_and_info: accoount_if_and_info[0], accounts_with_invalid_consumers_counter)))
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
    perform_accounts_state_checks(chain_connection=chain_ws_connection,
                                  accounts=all_accounts,
                                  chain_major_version=aleph_chain_version,
                                  locks=locks,
                                  bonded=bonded,
                                  ledger=ledger,
                                  next_keys=next_keys,
                                  contract_accounts=contract_accounts,
                                  block_hash=state_block_hash)
    log.info(f"DONE")
