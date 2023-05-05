BINDING_NAME ?= mpt-bindings

.PHONY: test
test:
	forge test --gas-report

.PHONY: proofgen
proofgen:
	rm -rf ./test/data/*
	cargo run --bin mpt-proof-gen -- --out ./test/data

.PHONY: bindgen
bindgen:
	forge bind --crate-name $(BINDING_NAME) --bindings-path ./crates/$(BINDING_NAME)
