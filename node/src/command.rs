use crate::{
    chain_spec,
    cli::{Cli, Subcommand},
    service,
};

#[cfg(feature = "runtime-benchmarks")]
pub use crate::benchmarking::{inherent_benchmark_data, RemarkBuilder, TransferKeepAliveBuilder};
#[cfg(feature = "runtime-benchmarks")]
pub use basednode_runtime::EXISTENTIAL_DEPOSIT;
#[cfg(feature = "runtime-benchmarks")]
pub use frame_benchmarking_cli::{BenchmarkCmd, ExtrinsicFactory, SUBSTRATE_REFERENCE_HARDWARE};
#[cfg(feature = "runtime-benchmarks")]
pub use sp_keyring::Sr25519Keyring;

use basednode_runtime::Block;
use sc_cli::{ChainSpec, RuntimeVersion, SubstrateCli};
use sc_service::PartialComponents;

impl SubstrateCli for Cli {
    fn impl_name() -> String {
        "Basednode Node".into()
    }

    fn impl_version() -> String {
        env!("SUBSTRATE_CLI_IMPL_VERSION").into()
    }

    fn description() -> String {
        env!("CARGO_PKG_DESCRIPTION").into()
    }

    fn author() -> String {
        env!("CARGO_PKG_AUTHORS").into()
    }

    fn support_url() -> String {
        "support.anonymous.an".into()
    }

    fn copyright_start_year() -> i32 {
        2017
    }

    fn load_spec(&self, id: &str) -> Result<Box<dyn sc_service::ChainSpec>, String> {
        Ok(match id {
            "local" => Box::new(chain_spec::localnet_config()?),
            "prometheus" => Box::new(chain_spec::prometheus_mainnet_config()?),
            "" | "test_cyan" => Box::new(chain_spec::cyan_testnet_config()?),
            path => Box::new(chain_spec::ChainSpec::from_json_file(
                std::path::PathBuf::from(path),
            )?),
        })
    }

    fn native_runtime_version(_: &Box<dyn ChainSpec>) -> &'static RuntimeVersion {
        &basednode_runtime::VERSION
    }
}

// Parse and run command line arguments
pub fn run() -> sc_cli::Result<()> {
    let cli = Cli::from_args();

    match &cli.subcommand {
        Some(Subcommand::Key(cmd)) => cmd.run(&cli),
        Some(Subcommand::BuildSpec(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.sync_run(|config| cmd.run(config.chain_spec, config.network))
        }
        Some(Subcommand::CheckBlock(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.async_run(|mut config| {
                let (client, _, import_queue, task_manager, _) =
                    service::new_chain_ops(&mut config, &cli.eth)?;
                Ok((cmd.run(client, import_queue), task_manager))
            })
        }
        Some(Subcommand::ExportBlocks(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.async_run(|mut config| {
                let (client, _, _, task_manager, _) =
                    service::new_chain_ops(&mut config, &cli.eth)?;
                Ok((cmd.run(client, config.database), task_manager))
            })
        }
        Some(Subcommand::ExportState(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.async_run(|mut config| {
                let (client, _, _, task_manager, _) =
                    service::new_chain_ops(&mut config, &cli.eth)?;
                Ok((cmd.run(client, config.chain_spec), task_manager))
            })
        }
        Some(Subcommand::ImportBlocks(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.async_run(|mut config| {
                let (client, _, import_queue, task_manager, _) =
                    service::new_chain_ops(&mut config, &cli.eth)?;
                Ok((cmd.run(client, import_queue), task_manager))
            })
        }
        Some(Subcommand::PurgeChain(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.sync_run(|config| cmd.run(config.database))
        }
        Some(Subcommand::Revert(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.async_run(|mut config| {
                let (client, backend, import_queue, task_manager, _) =
                    service::new_chain_ops(&mut config, &cli.eth)?;
                let aux_revert = Box::new(|client, _, blocks| {
                    sc_finality_grandpa::revert(client, blocks)?;
                    Ok(())
                });
                Ok((cmd.run(client, backend, Some(aux_revert)), task_manager))
            })
        }
        #[cfg(feature = "runtime-benchmarks")]
        Some(Subcommand::Benchmark(cmd)) => {
			Ok(())
            // let runner = cli.create_runner(cmd)?;
            //
            // runner.sync_run(|config| {
            //     // This switch needs to be in the client, since the client decides
            //     // which sub-commands it wants to support.
            //     match cmd {
            //         BenchmarkCmd::Pallet(cmd) => {
            //             if !cfg!(feature = "runtime-benchmarks") {
            //                 return Err(
            //                     "Runtime benchmarking wasn't enabled when building the node. \
			// 				You can enable it with `--features runtime-benchmarks`."
            //                         .into(),
            //                 );
            //             }
            //
            //             cmd.run::<Block, service::ExecutorDispatch>(config)
            //         }
            //         BenchmarkCmd::Block(cmd) => {
            //             let PartialComponents { client, .. } =
            //                 service::new_chain_ops(&mut config, &cli.eth)?;
            //             cmd.run(client)
            //         }
            //         #[cfg(not(feature = "runtime-benchmarks"))]
            //         BenchmarkCmd::Storage(_) => Err(
            //             "Storage benchmarking can be enabled with `--features runtime-benchmarks`."
            //                 .into(),
            //         ),
            //         #[cfg(feature = "runtime-benchmarks")]
            //         BenchmarkCmd::Storage(cmd) => {
            //             let PartialComponents {
            //                 client, backend, ..
            //             } = service::new_chain_ops(&mut config, &cli.eth)?;
            //             let db = backend.expose_db();
            //             let storage = backend.expose_storage();
            //
            //             cmd.run(config, client, db, storage)
            //         }
            //         BenchmarkCmd::Overhead(cmd) => {
            //             let PartialComponents { client, .. } =
            //                 service::new_chain_ops(&mut config, &cli.eth)?;
            //             let ext_builder = RemarkBuilder::new(client.clone());
            //
            //             cmd.run(
            //                 config,
            //                 client,
            //                 inherent_benchmark_data()?,
            //                 Vec::new(),
            //                 &ext_builder,
            //             )
            //         }
            //         BenchmarkCmd::Extrinsic(cmd) => {
            //             let PartialComponents { client, .. } =
            //                 service::new_partial(&config, &cli.eth)?;
            //             // Register the *Remark* and *TKA* builders.
            //             let ext_factory = ExtrinsicFactory(vec![
            //                 Box::new(RemarkBuilder::new(client.clone())),
            //                 Box::new(TransferKeepAliveBuilder::new(
            //                     client.clone(),
            //                     Sr25519Keyring::Alice.to_account_id(),
            //                     EXISTENTIAL_DEPOSIT,
            //                 )),
            //             ]);
            //
            //             cmd.run(client, inherent_benchmark_data()?, Vec::new(), &ext_factory)
            //         }
            //         BenchmarkCmd::Machine(cmd) => {
            //             cmd.run(&config, SUBSTRATE_REFERENCE_HARDWARE.clone())
            //         }
            //     }
            // })
        }
        #[cfg(feature = "try-runtime")]
        Some(Subcommand::TryRuntime(cmd)) => {
            use crate::service::ExecutorDispatch;
            use sc_executor::{sp_wasm_interface::ExtendedHostFunctions, NativeExecutionDispatch};
            let runner = cli.create_runner(cmd)?;
            runner.async_run(|config| {
                // we don't need any of the components of new_partial, just a runtime, or a task
                // manager to do `async_run`.
                let registry = config.prometheus_config.as_ref().map(|cfg| &cfg.registry);
                let task_manager =
                    sc_service::TaskManager::new(config.tokio_handle.clone(), registry)
                        .map_err(|e| sc_cli::Error::Service(sc_service::Error::Prometheus(e)))?;
                Ok((
                    cmd.run::<Block, ExtendedHostFunctions<
                        sp_io::SubstrateHostFunctions,
                        <ExecutorDispatch as NativeExecutionDispatch>::ExtendHostFunctions,
                    >>(),
                    task_manager,
                ))
            })
        }
        #[cfg(not(feature = "try-runtime"))]
        Some(Subcommand::TryRuntime) => Err("TryRuntime wasn't enabled when building the node. \
				You can enable it with `--features try-runtime`."
            .into()),
        Some(Subcommand::ChainInfo(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.sync_run(|config| cmd.run::<Block>(&config))
        }
        Some(Subcommand::FrontierDb(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.sync_run(|mut config| {
                let (client, _, _, _, frontier_backend) =
                    service::new_chain_ops(&mut config, &cli.eth)?;
                cmd.run(client, frontier_backend)
            })
        }
        None => {
            let runner = cli.create_runner(&cli.run)?;
            runner.run_node_until_exit(|config| async move {
                service::build_full(config, cli.eth).map_err(sc_cli::Error::Service)
            })
        }
    }
}
