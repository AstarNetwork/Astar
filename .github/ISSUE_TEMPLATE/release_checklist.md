---
name: Astar Release Checklist
about: Checklist to follow before creating a new release
title: 'Release v_xx Checklist'
labels:
assignees: ''
---

## Release v_xx Specific
<!---
All the preparation specific for this release, for example
- [x] Test batch precompile in Astar & Shiden
- [x] Test xtokens in Astar
-->


## Runtime Release
- [ ] Check Semver bumped for below crates
     - `astar-runtime`
     - `shiden-runtime`
     - `shibuya-runtime`
     - `local-runtime`
     - `astar-collator`
     - `xcm-tools` (if needed)
- [ ] Check Spec Version bumped for all runtimes
     - Astar: x -> y
     - Shiden: x -> y
     - Shibuya: x -> y
- [ ] Verify completed migrations are removed from any public networks.


## All Releases
- [ ]  Check and update new Github release is created with release logs.

## Post Release
- [ ] Notify Builders and DevOps
