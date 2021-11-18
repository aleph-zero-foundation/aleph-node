import logging
import os
import stat

from pathlib import Path
from shutil import copyfile


def copy_binary(binary: Path, target: str):
    logging.info(f'Copying binary from {binary} to {target}...')

    os.makedirs('bin', exist_ok=True)
    target = Path(f'bin/{target}')
    copyfile(binary, target)
    st = os.stat(target)
    os.chmod(target, st.st_mode | stat.S_IEXEC)

    logging.info(f'Copying binary succeeded.')
