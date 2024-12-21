#!/bin/bash

: "${CHAIN:=local}"
: "${BUILD_BINARY:=1}"
: "${SPEC_PATH:=specs/}"
: "${FEATURES:=pow-faucet}"

FULL_PATH="$SPEC_PATH$CHAIN.json"

if [ ! -d "$SPEC_PATH" ]; then
	echo "*** Creating directory ${SPEC_PATH}..."
	mkdir $SPEC_PATH
fi

if [[ $BUILD_BINARY == "1" ]]; then
	echo "*** Building substrate binary..."
	cargo build --release --features "$FEATURES"
	echo "*** Binary compiled"
fi

echo "*** Building chainspec..."
./target/release/basednode build-spec --disable-default-bootnode --raw --chain $CHAIN > $FULL_PATH
echo "*** Chainspec built and output to file"

echo "*** Purging previous state..."
./target/release/basednode purge-chain -y --base-path /tmp/bob --chain="$FULL_PATH" >/dev/null 2>&1
./target/release/basednode purge-chain -y --base-path /tmp/alice --chain="$FULL_PATH" >/dev/null 2>&1
echo "*** Previous chainstate purged"

echo "*** Starting localnet nodes..."
alice_start=(
	./target/release/basednode
	--base-path /tmp/alice
	--chain="$FULL_PATH"
	--alice
	--port 30334
	--ws-port 9946
	--rpc-port 9934
	--rpc-external
	--ws-external
	--rpc-methods Unsafe
	--ws-max-connections 10000
	--in-peers 500
	--out-peers 500
	--execution=native
	--wasm-execution=compiled
	--validator
	--rpc-cors=all
	--allow-private-ipv4
	--discover-local
)

bob_start=(
	./target/release/basednode
	--base-path /tmp/bob
	--chain="$FULL_PATH"
	--bob
	--port 30335
	--ws-port 9947
	--ws-external
	--rpc-external
	--rpc-port 9935
	--rpc-methods Unsafe
	--ws-max-connections 10000
	--in-peers 500
	--out-peers 500
	--execution=native
	--wasm-execution=compiled
	--rpc-cors=all
	--validator
	--discover-local
)

(trap 'kill 0' SIGINT; ("${alice_start[@]}" 2>&1) & ("${bob_start[@]}" 2>&1))
