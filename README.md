# solidity-mpt

This provides a verification library for Merkle Particia Tree(MPT) in Solidity.

You can see an example that verifies a proof of ERC-20 contract's state(i.e. account balance) [here](./test/MPTProof.t.sol).

## Generate proof data for testing

Note: the following command requires `ganache` in your PATH

```sh
$ cargo run --bin mpt-proof-gen -- --out ./test/data
```

## Acknowledgments

This library is a fork from https://github.com/lorenzb/proveth.
