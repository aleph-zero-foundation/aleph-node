# Cliain

A cmd line tool for interacting with the chain.

# Usage

## Overview

`cliain` is a wrapper over `substrate-api-client` library. It makes easy to call some Substrate chain
extrinsic or RPC calls. Run `./cliain --help` to see which of them are supported.

## Signing account

Tool reqires `--seed` parameter to sign given transaction with an account derived from the given seed.
If `--seed` is not given a prompt is displayed to enter the seed.

## WS endpoint

Bu default tool connects to 127.0.0.1:9944 port, and this can be controller by `--node` flag.
