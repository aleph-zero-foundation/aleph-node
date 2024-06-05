import logging
import datetime


def setup_global_logger():
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
