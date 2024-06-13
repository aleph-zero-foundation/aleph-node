import logging
import itertools
import functools

from chain_operations import format_balance, get_all_accounts

log = logging.getLogger()


def get_total_issuance_imbalance(chain_connection, block_hash):
    """
    Compares total issuance computed from all accounts with balances.total_issuance storage
    :param chain_connection: WS handler
    :param block_hash: total issuance computed from all accounts
    :return: delta between those two values
    """
    total_issuance_from_chain = get_total_issuance_from_storage(chain_connection=chain_connection,
                                                                block_hash=block_hash)
    all_accounts_and_infos = get_all_accounts(chain_connection, block_hash)
    total_issuance_from_accounts = calculate_total_issuance(
        map(lambda account_and_info: account_and_info[1], all_accounts_and_infos))
    return total_issuance_from_chain, total_issuance_from_accounts


def log_total_issuance_imbalance(chain_connection,
                                 total_issuance_from_chain,
                                 total_issuance_from_accounts,
                                 block_hash):
    """
    Logs imbalance data in a given block hash in a human-readable format.
    :param chain_connection: WS handler
    :param total_issuance_from_chain: balances.total_issuance storage value
    :param total_issuance_from_accounts: total_issuance as sum aggregated over all accounts
    :param block_hash: block hash from which above data was retrieved
    :return: None
    """
    total_issuance_from_accounts_human = format_balance(chain_connection, total_issuance_from_accounts)
    total_issuance_from_chain_human = format_balance(chain_connection, total_issuance_from_chain)
    delta = total_issuance_from_chain - total_issuance_from_accounts
    delta_human = format_balance(chain_connection, delta)
    log.info(f"Total issuance imbalance computed from block {block_hash}")
    log.info(
        f"balances.total_issuance storage value: {total_issuance_from_chain_human}")
    log.info(
        f"Total issuance computed as aggregated sum over all accounts: {total_issuance_from_accounts_human}")
    log.info(f"Delta is: {delta_human}")


def calculate_total_issuance(account_infos):
    """
    Calculates total issuance as sum over all accounts free + reserved funds
    :param account_infos: A list AccountInfo structs
    :return: total issuance as number
    """

    def get_account_total_balance(account_info):
        free = account_info['data']['free']
        reserved = account_info['data']['reserved']
        return free + reserved

    return \
        functools.reduce(lambda x, account_info: x + get_account_total_balance(account_info), account_infos, 0)


def get_total_issuance_from_storage(chain_connection, block_hash):
    """
    Retrieves balances.total_issuance StorageValue
    :param chain_connection: WS handler
    :param block_hash: A block hash to query state from
    :return: total issuance as number
    """
    total_issuance_from_chain = chain_connection.query(module='Balances',
                                                       storage_function='TotalIssuance',
                                                       block_hash=block_hash).value
    return total_issuance_from_chain


def find_block_hash_with_imbalance(chain_connection, start_block_hash, end_block_hash):
    """
    Finds a first block hash that positively contributed to a total issuance imbalance.

    Positive contribution to the total issuance imbalance in block B is a situation in which total issuance imbalance
    increases in block B. Total issuance imbalance is a difference between aggregated sum of total bolance over all
    accounts and balances.total_issuance storage value. It might happen that difference in some value X in block B,
    and some value Y in parent(B), and X > Y. There might be many such blocks in chain [start_block_hash; end_block_hash]
    and this method returns the first one.

    Method uses bisection algorithm. It computes mid-range block hash by computing
      mid_block_number = floor((end_block_number - start_block_number) / 2)
    and then calculating total_issuance imbalance in mid_block_number to start and end range total_issuance imbalance,
    adjusting interval ends accordingly to bisection algorith.

    :param chain_connection: WS handler
    :param start_block_hash: first block hash in range to check
    :param end_block_hash: end block hash in range to check
    :return: the first block_hash that contributed positively to total issuance imbalance
    """
    start_block_number = chain_connection.get_block_number(start_block_hash)
    end_block_number = chain_connection.get_block_number(end_block_hash)

    start_total_issuance_imbalance = get_total_issuance_imbalance(chain_connection, start_block_hash)
    log_total_issuance_imbalance(chain_connection=chain_connection,
                                 total_issuance_from_chain=start_total_issuance_imbalance[0],
                                 total_issuance_from_accounts=start_total_issuance_imbalance[1],
                                 block_hash=start_block_hash)
    delta_start_imbalance = start_total_issuance_imbalance[0] - start_total_issuance_imbalance[1]

    while end_block_number - 1 > start_block_number:
        log.info(f"Finding first block that contributed to total issuance imbalance in range "
                 f"[{start_block_number}; {end_block_number}]")

        mid_range_block_number = start_block_number + (end_block_number - start_block_number) // 2
        mid_range_block_hash = chain_connection.get_block_hash(mid_range_block_number)
        log.info(f"Mid-range block hash: {mid_range_block_hash}, number: {mid_range_block_number}")
        mid_total_issuance_imbalance = get_total_issuance_imbalance(chain_connection, mid_range_block_hash)
        log_total_issuance_imbalance(chain_connection=chain_connection,
                                     total_issuance_from_chain=mid_total_issuance_imbalance[0],
                                     total_issuance_from_accounts=mid_total_issuance_imbalance[1],
                                     block_hash=mid_range_block_hash)

        delta_mid_imbalance = mid_total_issuance_imbalance[0] - mid_total_issuance_imbalance[1]
        if delta_mid_imbalance > delta_start_imbalance:
            end_block_hash = mid_range_block_hash
            end_block_number = chain_connection.get_block_number(end_block_hash)
        else:
            start_block_hash = mid_range_block_hash
            start_block_number = chain_connection.get_block_number(start_block_hash)
            delta_start_imbalance = delta_mid_imbalance

    return chain_connection.get_block_hash(end_block_number)
