This directory gathers useful actions for Github pipelines.

## `restore-cache`
This action restores and saves cache dedicated for Rust builds
(in particular we keep Cargo files, target directories and make use of sccache).

### Usage
Sample usage:
```yaml
steps:
- uses: Cardinal-Cryptography/github-actions/restore-cache@cache-v1

- run: cargo build
  
- uses: Cardinal-Cryptography/github-actions/post-cache@v1
```

For building packages excluded from main workspace, you can add corresponding input parameter:
```yaml
steps:
- uses: Cardinal-Cryptography/github-actions/restore-cache@cache-v1
  with:
    cargo-targets: |
      excluded_package_1/target/
      aux_tool/excluded_package_2/target/
```

### Notes

**Note:** Currently we only support runners using `ubuntu:latest` image.

**Note:** Requires calling `post-cache` action to stop sccache server.

**Note:** There is a problem with using `cargo clippy` with `sccache` - check: https://github.com/mozilla/sccache/issues/966.
Effectively, you have to override environment variables like this:
```shell
export RUSTC_WRAPPER=""
export RUSTC_WORKSPACE_WRAPPER=sccache
cargo clippy
```

---

## `post-cache`
Stops sccache server. Use together with `restore-cache`.
