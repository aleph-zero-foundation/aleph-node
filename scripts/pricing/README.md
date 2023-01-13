Pricing script
==============

The `./run.py` script in this directory will deploy some contracts and print a summary of how much some basic operations
on them cost.

It requires `python3` and an Ink 4-compatible version of `cargo contract`, to install:

```bash
$ cargo install cargo-contract --version 2.0.0-beta.1
```

Afterwards, install the python deps and run the script:

```bash
$ pip install -r requirements.txt
$ ./run.py
```

For more info on options see:

```bash
$ ./run.py --help
```
