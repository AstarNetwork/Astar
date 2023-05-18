# GitHub Actions

## Benchmarks
Benchmark workflow does pallet benchmarks automatically when detecting PR comments start with `/bench`.
As of now, it is a simple workflow, users can get artifacts (pallet benchmarks results & machine benchmark result) as downloadale artifacts of Github actions.
Latest commit hash in PR branch will be built and used for benchmarking.

### How to execute
Usage
```
# [chain name] - astar-dev, shiden-dev, shibuya-dev, dev
# [pallet names] - Use "," for multiple pallets, "all" for all pallets
/bench [chain name] [pallet names]
```
```
# benchmark a pallet
/bench astar-dev pallet_balances

# benchmark multiple pallets
/bench astar-dev pallet_balances,pallet_dapps_staking

# benchmark all pallets
/bench astar-dev all
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
