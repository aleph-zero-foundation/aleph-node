import json
import logging
import pprint

log = logging.getLogger()


def save_accounts_to_json_file(json_file_name, accounts):
    log.info(f"Saving accounts to json file {json_file_name}")
    with open(json_file_name, 'w') as f:
        log.info(f"Generating pretty print...")
        pretty_json_str = pprint.pformat(accounts, compact=True).replace("'",'"')
        f.write(pretty_json_str)
        log.info(f"Wrote file '{json_file_name}'")


def chunks(list_of_elements, n):
    """
    Lazily split 'list_of_elements' into 'n'-sized chunks.
    """
    for i in range(0, len(list_of_elements), n):
        yield list_of_elements[i:i + n]


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
