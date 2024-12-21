cargo build --release --features runtime-benchmarks
./target/release/basednode build-spec --disable-default-bootnode --raw --chain local > temp.json
./target/release/basednode benchmark pallet --chain=temp.json --execution=native  --wasm-execution=compiled --pallet pallet-basednode --extrinsic 'benchmark_dissolve_network' --output benchmarking.txt
rm temp.json