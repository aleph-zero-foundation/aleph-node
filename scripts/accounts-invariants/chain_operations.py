import substrateinterface
from tqdm import tqdm
import sys
import logging

log = logging.getLogger()


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

    for (i, (account_id, info)) in tqdm(iterable=enumerate(account_query),
                                        desc="Accounts checked",
                                        unit="",
                                        file=sys.stdout):
        total_accounts_count += 1
        if check_accounts_predicate(info, chain_major_version, ed):
            accounts_that_do_meet_predicate.append([account_id.value, info.serialize()])

    log.info(
        f"Total accounts that match given predicate {check_accounts_predicate_name} is {len(accounts_that_do_meet_predicate)}")
    log.info(f"Total accounts checked: {total_accounts_count}")
    return accounts_that_do_meet_predicate


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
                           block_hash=block_hash)
