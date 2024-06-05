import logging
from tqdm import tqdm
import sys

log = logging.getLogger()


def query_contract_and_code_owners_accounts(chain_connection, block_hash):
    """
    Returns contract accounts and code owners.
    """
    code_owners = set()
    contract_accounts = set()

    log.info(f"Querying code owners.")
    code_info_of_query = chain_connection.query_map(module='Contracts',
                                                    storage_function='CodeInfoOf',
                                                    page_size=500,
                                                    block_hash=block_hash)

    for (i, (account_id, info)) in tqdm(iterable=enumerate(code_info_of_query),
                                        desc="CodeInfoOfs checked",
                                        unit="",
                                        file=sys.stdout):
        code_owners.add(info.serialize()['owner'])

    log.info(f"Total code owners is {len(code_owners)}")
    log.debug(f"Code owners: {code_owners}")

    log.info(f"Querying contract accounts.")
    contract_info_of_query = chain_connection.query_map(module='Contracts',
                                                        storage_function='ContractInfoOf',
                                                        page_size=1000,
                                                        block_hash=block_hash)
    for (i, (account_id, info)) in tqdm(iterable=enumerate(contract_info_of_query),
                                        desc="ContractInfoOfs checked",
                                        unit="",
                                        file=sys.stdout):
        contract_accounts.add(account_id.value)

    log.info(f"Total contracts count is {len(contract_accounts)}")
    log.debug(f"Contract accounts: {contract_accounts}")

    return code_owners, contract_accounts
