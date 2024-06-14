import substrateinterface
from tqdm import tqdm
import sys
import logging
from utils import chunks, format_balance
import pprint

log = logging.getLogger()


def get_all_accounts(chain_connection, block_hash):
    """
    Retrieves all accounts from an AlephNode chain (ie System.Account storage map)
    :param chain_connection: WS handler
    :param block_hash: A block hash to query state from
    :return: A list of two elem lists (AccountId, AccountInfo)
    """
    all_accounts = []
    query_storage_map(chain_connection=chain_connection,
                      pallet="System",
                      storage_map="Account",
                      block_hash=block_hash,
                      output_container=all_accounts,
                      output_functor=lambda account_id, storage_value, output_container:
                      output_container.append((account_id.value, storage_value.serialize()))
                      )
    return all_accounts


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


def batch_fix_accounts_consumers_counter(chain_connection,
                                         input_args,
                                         accounts,
                                         sender_keypair):
    """
    Send operations.fix_accounts_consumers_counter call in a batch
    :param chain_connection: WS connection handler
    :param input_args: script input arguments returned from argparse
    :param accounts: list of accounts to fix their consumers counter
    :param sender_keypair: keypair of sender account
    :return: None. Can raise exception in case of SubstrateRequestException thrown
    """
    for (i, account_ids_chunk) in tqdm(iterable=enumerate(chunks(accounts, input_args.fix_accounts_in_batch)),
                                       desc="Accounts checked",
                                       unit="",
                                       file=sys.stdout):
        operations_calls = list(map(lambda account: chain_connection.compose_call(
            call_module='Operations',
            call_function='fix_accounts_consumers_counter',
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
        log.info(f"About to send {len(operations_calls)} Operations.fix_accounts_consumers_counter, "
                 f"from {sender_keypair.ss58_address} to below accounts: "
                 f"{account_ids_chunk}")

        submit_extrinsic(chain_connection, extrinsic, len(operations_calls), input_args.dry_run)


def query_storage_map(chain_connection, pallet, storage_map, block_hash, output_container, output_functor):
    log.info(f"Querying {pallet}.{storage_map}")
    query_map_request = chain_connection.query_map(module=pallet,
                                                   storage_function=storage_map,
                                                   page_size=1000,
                                                   block_hash=block_hash)
    for account_id, storage_value in tqdm(iterable=query_map_request,
                                          desc="Entries checked",
                                          unit="",
                                          file=sys.stdout):
        output_functor(account_id, storage_value, output_container)
    log.debug(f"{pallet}.{storage_map} size: {len(output_container)}")
    log.info(f"Generating pretty print for {pallet}.{storage_map}...")
    pretty_json_str = pprint.pformat(output_container, compact=True).replace("'",'"')
    log.debug(f"{pallet}.{storage_map} data: {pretty_json_str}")
