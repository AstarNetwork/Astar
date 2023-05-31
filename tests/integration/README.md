# Integration tests

## Overview

Instead of mocks, integration tests covers tests on production runtimes, including Shibuya, Shiden and Astar. It's expected tests are added to cover major custom configurations in runtime, for instance `pallet-proxy` settings.

## Usages

To run integration tests for a specific runtime, for instance, Shibuya:

```shell
cargo test -p integration-tests --features=shibuya
```

To run integration tests for all runtimes:

```shell
make test-runtimes
```

## Development guidelines

General imports and configures that are shared across tests should be added to `setup.rs`. When new pallets are added to runtime, their hooks need to be checked and added to `run_to_block` if needed.

For specific tests like `pallet-proxy`, group them in one source file like `proxy.rs`. Then add the module to `lib.rs` with proper features config.
