import logging
from chain_operations import query_storage_map

log = logging.getLogger()


def query_contract_and_code_owners_accounts(chain_connection, block_hash):
    """
    Returns contract accounts and code owners.
    """
    code_owners = set()
    contract_accounts = set()

    query_storage_map(chain_connection=chain_connection,
                      pallet="Contracts",
                      storage_map="CodeInfoOf",
                      block_hash=block_hash,
                      output_container=code_owners,
                      output_functor=lambda account_id, storage_value, output_container:
                      output_container.add(storage_value.serialize()['owner'])
                      )
    query_storage_map(chain_connection=chain_connection,
                      pallet="Contracts",
                      storage_map="ContractInfoOf",
                      block_hash=block_hash,
                      output_container=contract_accounts,
                      output_functor=lambda account_id, storage_value, output_container:
                      output_container.add(account_id.value)
                      )

    return code_owners, contract_accounts