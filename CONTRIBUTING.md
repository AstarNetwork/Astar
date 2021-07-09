# Contributing

Welcome potential contributor of `Astar Network`! The Astar Network project (formerly known as Plasm Network) is a collection of **Open Source Projects** maintained by the Astar Team and Stake Technologies. We want to make contributing to this project as easy and transparent as possible.

This document will act as a starting point for those who want to be part of the Astar Ecosystem, which includes code contribution and community contribution.

## Types of Contribution

We welcome any types of contributions that can improve the project/network in any shape or form, may it be directly to the Astar repository codebase, feedback, or making community contributions. You don't have to be a developer to contribute to the network.

## Using GitHub

The Astar Network project uses GitHub as the main source control hosting service. Most forms of communication regarding changes to the code will be done within the issue board of the repository.

### Opening an Issue

Contributions within GitHub can take on the following forms:

- **Bug Report**: If you find any bugs or unexpected behaviors using the Astar node, please open an issue that describes the issue and other information that the developer may need when investigating.
- **User Questions**: Posting your question that is not addressed on our [official docs](https://docs.plasmnet.io/), [Substrate docs](https://substrate.dev/docs/en/), or through other issue tickets helps us improve our wiki and keep the community informed. For any inquiries related to the usage/development of the code, please open an issue on our repository.
- **Feature Request**: If you have any suggestions or requests for a feature that can be made within a *relatively short development time*, feel free to open an issue that describes the 'what,' 'why,' 'how,' of the feature.

### Code Changes (Pull Request)

For those who wish to make changes to the source code, please ensure that there is an open issue that is related to the changes you're trying to make. *You must open an issue before you open a pull request* as this allows us to understand what changes will come and prevent stale pull requests. The issue should contain a rough description of how you are planning on changing the code to fix or add features. Once the repository maintainer gives the green light, you can fork the repository and open a pull request with your changes to our main branch (Dusty).

In short:

1. Open an issue regarding a bug fix or feature request (fill in our issue templates)
2. Briefly describe how you plan to make changes to the code
3. Fork our main branch (Dusty)
4. Open a pull request to the main branch (fill in our pull request template)
5. Ensure all workflow checks have passed
6. Wait for the maintainers approval or change requests
7. Your code will be merged

### Coding Styles

Contributors should adhere to the [house coding style](https://substrate.dev/recipes/) and the [`rustfmt` styles](https://github.com/rust-lang/rustfmt).

### Branch Rules and Release Process

![branch-release](https://mermaid.ink/img/eyJjb2RlIjoiZ3JhcGggVERcbiAgICBGRUFUVVJFW2ZlYXR1cmUgYnJhbmNoXSAtLT58QWRkcyBuZXcgZmVhdHVyZXwgUFIocHVsbCByZXF1ZXN0KVxuICAgIEZJWFtmaXggYnJhbmNoXSAtLT58Rml4ZXMgYnVnfCBQUihwdWxsIHJlcXVlc3QpXG4gICAgRE9DW2RvY3VtZW50IGJyYW5jaF0gLS0-fEFkZHMgZG9jdW1lbnRhdGlvbnwgUFIob3BlbiBwdWxsIHJlcXVlc3QpXG4gICAgUFIgLS0-fEludGVybmFsIHRlc3RpbmcgJiBNZXJnZSBicmFuY2h8IERFVltkZXZlbG9wbWVudCBicmFuY2hdXG4gICAgREVWIC0tPiB8UmVsZWFzZSAmIERlcGxveSB0ZXN0IG5ldHwgVEVTVChwdWJsaWMgdGVzdGluZylcbiAgICBURVNUIC0tPiB8SW1wcm92ZW1lbnRzfCBQUlxuICAgIFRFU1QgLS0-IHxPcGVuIHB1bGwgcmVxdWVzdHwgUkVMRUFTRVtwcm9kdWN0aW9uIGJyYW5jaF0iLCJtZXJtYWlkIjp7fSwidXBkYXRlRWRpdG9yIjpmYWxzZSwiYXV0b1N5bmMiOnRydWUsInVwZGF0ZURpYWdyYW0iOmZhbHNlfQ)

All branch names should adhere to the following rules:

- `production/*`: production/release version of the node will have the `production/` prefix in their branch name. These branches will be protected and no direct pushes will be allowed.
- `development/*`: nodes that are actively in development (including release candidates) will have the `development/` prefix in their branch name.

Every major features made for the `development` branch must go through at least one week of internal testing before it is released and deployed.

Due to the different base runtime version for each chain, we need to maintain Astar Ecosystem in separate branches.
We will be improving this project structure in the near future, but to maintain network stability, major runtime upgrades will follow this process:

- `development/dusty` → `production/astar` (independent blockchain network planed to be a Parachain of Polkadot network)
- `development/shibuya` → `production/shiden` (Parachain of Kusama and focused on cutting edge XCMP development)

### Contributor Licenses

By contributing, you agree that your contributions will be licensed under the [GNU General Public License v3.0](https://github.com/PlasmNetwork/Plasm/blob/development/dusty/LICENSE) as is with the Astar source code. If you have any concerns regarding this matter, please contact the maintainer.

## Community Contribution

As a public blockchain network, Astar Network welcomes any contributions that help our community members and create the best blockchain network. Anyone can interact with the community through our [official forum](https://forum.plasmnet.io/), [discord](https://discord.gg/aApeT8r2e4), and [Telegram](https://t.me/PlasmOfficial). You can contribute to the community by actively participating in the forum discussions, helping other members, or sharing Astar Network with others.
