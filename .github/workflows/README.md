# GitHub Actions

## Benchmarks
Benchmark workflow does pallet benchmarks automatically when detecting PR comments start with `/bench`.
As of now, it is a simple workflow, users can get artifacts (pallet benchmarks results & machine benchmark result) as downloadale artifacts of Github actions.
Latest commit hash in PR branch will be built and used for benchmarking.

### How to execute
Usage
```
# [chain names] - Use "," for multiple runtimes. Available values are: astar, shiden, shibuya
# [pallet names] - Use "," for multiple pallets, "all" for all pallets
/bench [chain names] [pallet names]
```
```
# benchmark a pallet
/bench astar pallet_balances

# benchmark multiple pallets
/bench astar pallet_balances,pallet_dapps_staking

# benchmark all pallets
/bench astar all

# benchmark multiple runtimes with multiple pallets
/bench astar,shibuya pallet_balances,pallet_dapps_staking
```


### Reference machine
```
Hardware
CPU
Intel Xeon-E 2136 - 6c/12t - 3.3 GHz/4.5 GHz
RAM
32 GB ECC 2666 MHz

Data disks
2×500 GB SSD NVMe

Expansion cards
Soft RAID
```

## Runtime upgrade test

Runtime upgrade test workflow does test automatically when detecting PR comments start with `/runtime-upgrade-test`.

### How to execute

Usage:

```
# [runtime] - shibuya, shiden, astar
/runtime-upgrade-test shibuya
```
