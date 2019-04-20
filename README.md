<div align="center"><img width="300" alt="plasm" src="https://user-images.githubusercontent.com/6259384/55708398-cf9ae900-5a20-11e9-859c-3435b55c68a5.png"></div>

Plasm is a Substrate Runtime Module Library by which a developer can add Plasma functions to his/her own Substrate chain.

__IMPORTANT NOTICE:__
* __This is an experimental software, does not run in a production yet.__

## Table of Contents
- Introduction
- Background
- Plasm
    - Plasm-UTXO
    - Plasm-Parent
    - Plasm-Child
- How to Install


## Background
Today, there are many derived Plasmas. For example, 

Plasma には複数の種類、派生系が存在する。例えば初めに Vitanik によって提唱された Plasma-MVP, MVP の不正申告者が子チェーンのフルノードを持たなければならない問題を解決した Plasma-Cash その派生系である Plasma-XT, Prime。そして Plasma-Prime を参考に実装された Plasma-Chamber。ZK-S[T|N]ARKSを用いた Plasma-Snapps 等があげられる。

Plasm では複数の Plasma Solution をプラガブルに組み合わせて使用できるような Plasma-Abstract なデータ構造と各々の Plasma ソリューションに対応する Rust on Substrate 実装を提供する。

Substrate developers can import one of Plasm Libraries and make thier own plasma chain depending on their use casse. Plasm consists of 3 (or 4) libraries, plasm-utxo, plasm-parent and plasm-child.  


plasm-utxo は Deposit したトークン/コイン(以後トークンで統一)を管理するための UTXO-like なデータ構造を提供する。Plasma ではトークンを親チェーンに Exit する際の不正申告のためにトークンの取引履歴を保持する必要がある。何故ならばあるトークンが不正に Exit されたことを証明するにはそのトークンを不正 Exitor が保持していないことを示すために正しい取引履歴を示す必要があるからだ。plasm-utxo では抽象化された UTXOs とそれの具象実装が各 Plasma ソリューションについて実装される。
plasm-parent は Plasma の親チェーンとして動作させるためのモジュールを提供する。
plasm-child は Plasma の子チェーンとして動作させるためのモジュールを提供する。

## Plasm-UTXO
Plasm-UTXO は各 Plasma ソリューションに適したトランザクションの仕様を提供する。
Along with that, we 
また、それに伴い UTXO-like なデータ構造全般を網羅的に扱えるような設計をしている。また、Merkle Tree を内包しておりこれについても着脱可能である。

## Plasm-Parent
Plasm-Parent provides a specification of the parent chain. 子チェーンには親チェーンの各種ソリューションに対応する実装がされており、これらをセットで使うことで親子間の取引を実現することができる。主に各種 Exit Game についてのロジックを実装する。

## Plasm-Child
Plasm-Child provides a specification of the child chain. Parent chain has been implemented corresponding to the child chain's solutions. 

By using these solutions together, the user can make the transactions happen between the parent chain and the child chain. The logic of "deposit/exit" has been implemented based on Plasm-UTXO.

# How to install

## UTXO
```toml
[dependencies.utxo]
git = 'https://github.com/stakedtechnologies/Plasm.git'
package = 'plasm-utxo'
version = '0.1.0' 
```

## Parent
Comming soon...

## Child
Comming soon...

* * *
Plasm is licensed under the Apache License, Version2.0 by Staked Technologies Inc.
