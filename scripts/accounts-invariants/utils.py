import json
import logging

log = logging.getLogger()


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
