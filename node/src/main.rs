//! Substrate Node Basednode CLI library.
#![warn(missing_docs)]

mod chain_spec;
#[macro_use]
mod service;
#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
mod cli;
mod client;
mod command;
mod eth;
mod rpc;

fn main() -> sc_cli::Result<()> {
    command::run()
}
