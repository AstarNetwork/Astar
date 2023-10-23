---
name: Astar Release Checklist
about: Checklist to follow before creating a new release
title: 'Release v_xx Checklist'
labels:
assignees: ''
---

## Required Changes (PR)
<!---
All the PRs that should, For Example,
- [x] #1000
- [x] #1006 
-->

## Release v_xx Specific
<!---
All the preparation specific for this release, for example
- [x] Test batch precompile in Astar & Shiden
- [x] Test xtokens in Astar
-->


## Runtime Release
- [ ] Check Semver bumped
     - Current: v_xx
     - Last: v_xx
- [ ] Check Spec Version bumped for all runtimes
     - Astar: x -> y
     - Shiden: x -> y
     - Shibuya: x -> y
- [ ] Verify completed migrations are removed from any public networks.
- [ ] Verify new extrinsics have been correctly whitelisted/blacklisted for proxy filters.
- [ ] Verify benchmarks & weights have been updated for any modified runtime logics.


## All Releases
- [ ]  Check and update new Github release is created with release logs.

## Post Release
- [ ] Notify Builders and DevOps
