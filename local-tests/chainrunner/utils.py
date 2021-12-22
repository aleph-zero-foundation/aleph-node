import os.path as op
import re
import subprocess
from collections import OrderedDict


def generate_keys(binary, phrases):
    """Generate public keys based on the list of seed phrases.
    `binary` should be a path to some `aleph-node` binary. `phrases` should be a list of strings.
    Returns an ordered dictionary with phrases as keys and corresponding public keys as values.
    The order follows the order in `phrases`.
    """
    check_file(binary)
    regexp = re.compile(r'SS58 Address:\s*(\w+)$', re.MULTILINE)
    res = OrderedDict()
    for p in phrases:
        out = subprocess.check_output([binary, 'key', 'inspect', p]).decode()
        matches = regexp.findall(out)
        res[p] = matches[0] if matches else None
    return res


def check_file(path):
    """Ensure the provided path points to an existing file."""
    if not op.isfile(path):
        raise FileNotFoundError(f'file not found: {path}')
    return path


def flag(s):
    """Turn 'flag_name' into `--flag-name`."""
    return '--' + str(s).replace('_', '-')


def flags_from_dict(d):
    """Turn a dictionary of flags into a list of strings required by subprocess methods."""
    res = []
    for k,v in d.items():
        res.append(flag(k))
        if v is not True:
            res.append(str(v))
    return res
