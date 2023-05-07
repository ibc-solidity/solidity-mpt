# solidity-mpt

## Disclaimer

This package has not been audited. The developers take no responsiblity should this package be used in a production environment without first auditing sections applicable to you by a reputable auditing firm.

## Introduction

This provides a verification library for Ethereum's Merkle Particia Tree(MPT) in Solidity.

You can see an example that verifies a proof of ERC-20 contract's state(i.e. account balance) [here](./test/MPTProof.t.sol).

## Generate proof data for testing

Note: the following command requires `ganache` in your PATH

```sh
$ cargo run --bin mpt-proof-gen -- --out ./test/data
```

## Acknowledgments

This library is a fork from https://github.com/lorenzb/proveth.
