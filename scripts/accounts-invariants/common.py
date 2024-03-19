#!/bin/python3

import enum
import logging
import datetime

import substrateinterface
import json

def get_global_logger():
    log_formatter = logging.Formatter("%(asctime)s [%(levelname)s] %(message)s")
    root_logger = logging.getLogger()
    root_logger.setLevel('DEBUG')

    time_now = datetime.datetime.now().strftime("%d-%m-%Y_%H:%M:%S")
    file_handler = logging.FileHandler(f"pallet-balances-maintenance-{time_now}.log")
    file_handler.setFormatter(log_formatter)
    file_handler.setLevel(logging.DEBUG)
    root_logger.addHandler(file_handler)

    console_handler = logging.StreamHandler()
    console_handler.setFormatter(log_formatter)
    console_handler.setLevel(logging.INFO)
    root_logger.addHandler(console_handler)

    return logging


log = get_global_logger()


class ChainMajorVersion(enum.Enum):
    PRE_12_MAJOR_VERSION = 65,
    AT_LEAST_12_MAJOR_VERSION = 68,
    AT_LEAST_13_2_VERSION = 71,

    @classmethod
    def from_spec_version(cls, spec_version):
        if spec_version <= 65:
            return cls(ChainMajorVersion.PRE_12_MAJOR_VERSION)
        elif 68 <= spec_version < 71 or spec_version == 72:
            return cls(ChainMajorVersion.AT_LEAST_12_MAJOR_VERSION)
        elif spec_version >= 71:
            return cls(ChainMajorVersion.AT_LEAST_13_2_VERSION)


def get_chain_major_version(chain_connection, block_hash):
    """
    Retrieves spec_version from chain and returns an enum whether this is pre 12 version or at least 12 version
    :param chain_connection: WS handler
    :param block_hash: Block hash to query state from
    :return: ChainMajorVersion
    """
    runtime_version = chain_connection.get_block_runtime_version(block_hash)
    spec_version = runtime_version['specVersion']
    major_version = ChainMajorVersion.from_spec_version(spec_version)
    return major_version


def filter_accounts(chain_connection,
                    ed,
                    chain_major_version,
                    check_accounts_predicate,
                    check_accounts_predicate_name="",
                    block_hash=None):
    """
    Filters out all chain accounts by given predicate.
    :param chain_connection: WS handler
    :param ed: existential deposit
    :param chain_major_version: enum ChainMajorVersion
    :param check_accounts_predicate: a function that takes three arguments predicate(account, chain_major_version, ed)
    :param check_accounts_predicate_name: name of the predicate, used for logging reasons only
    :param block_hash: A block hash to query state from
    :return: a list which has those chain accounts which returns True on check_accounts_predicate
    """
    accounts_that_do_meet_predicate = []
    # query_map reads state from the **single** block, if block_hash is not None this is top of the chain
    account_query = chain_connection.query_map(module='System',
                                               storage_function='Account',
                                               page_size=1000,
                                               block_hash=block_hash)
    total_accounts_count = 0
    total_issuance = 0

    for (i, (account_id, info)) in enumerate(account_query):
        total_accounts_count += 1
        free = info['data']['free'].value
        reserved = info['data']['reserved'].value
        total_issuance += free + reserved
        if check_accounts_predicate(info, chain_major_version, ed):
            accounts_that_do_meet_predicate.append([account_id.value, info.serialize()])
        if i % 5000 == 0 and i > 0:
            log.info(f"Checked {i} accounts")

    log.info(
        f"Total accounts that match given predicate {check_accounts_predicate_name} is {len(accounts_that_do_meet_predicate)}")
    log.info(f"Total accounts checked: {total_accounts_count}")
    return accounts_that_do_meet_predicate, total_issuance


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


def get_all_accounts(chain_connection, block_hash=None):
    return filter_accounts(chain_connection=chain_connection,
                           ed=None,
                           chain_major_version=None,
                           check_accounts_predicate=lambda x, y, z: True,
                           check_accounts_predicate_name="\'all accounts\'",
                           block_hash=block_hash)[0]


def save_accounts_to_json_file(json_file_name, accounts):
    with open(json_file_name, 'w') as f:
        json.dump(accounts, f)
        log.info(f"Wrote file '{json_file_name}'")


def chunks(list_of_elements, n):
    """
    Lazily split 'list_of_elements' into 'n'-sized chunks.
    """
    for i in range(0, len(list_of_elements), n):
        yield list_of_elements[i:i + n]
