// This file is part of Bifrost.

// Copyright (C) 2019-2022 Liebi Technologies (UK) Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use std::{io::Write, net::SocketAddr};

use codec::Encode;
use cumulus_client_service::genesis::generate_genesis_block;
use cumulus_primitives_core::ParaId;
use frame_benchmarking_cli::BenchmarkCmd;
use log::info;
use node_service::{self as service, IdentifyVariant};
use polkadot_parachain::primitives::AccountIdConversion;
use sc_cli::{
	ChainSpec, CliConfiguration, DefaultConfigurationValues, ImportParams, KeystoreParams,
	NetworkParams, Result, RuntimeVersion, SharedParams, SubstrateCli,
};
use sc_service::config::{BasePath, PrometheusConfig};
#[cfg(any(feature = "with-bifrost-polkadot-runtime", feature = "with-bifrost-runtime"))]
use service::collator_polkadot::BifrostPolkadotExecutor;
use sp_core::hexdisplay::HexDisplay;
use sp_runtime::traits::Block as BlockT;

use crate::{Cli, RelayChainCli, Subcommand};

fn get_exec_name() -> Option<String> {
	std::env::current_exe()
		.ok()
		.and_then(|pb| pb.file_name().map(|s| s.to_os_string()))
		.and_then(|s| s.into_string().ok())
}

fn load_spec(id: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
	let id = if id.is_empty() {
		let n = get_exec_name().unwrap_or_default();

		["bifrost"]
			.iter()
			.cloned()
			.find(|&chain| n.starts_with(chain))
			.unwrap_or("bifrost")
	} else {
		id
	};
	Ok(match id {
		#[cfg(any(feature = "with-bifrost-kusama-runtime", feature = "with-bifrost-runtime"))]
		"bifrost" | "bifrost-kusama" =>
			Box::new(service::chain_spec::bifrost_kusama::ChainSpec::from_json_bytes(
				&include_bytes!("../../service/res/bifrost-kusama.json")[..],
			)?),
		#[cfg(any(feature = "with-bifrost-kusama-runtime", feature = "with-bifrost-runtime"))]
		"bifrost-genesis" | "bifrost-kusama-genesis" =>
			Box::new(service::chain_spec::bifrost_kusama::chainspec_config()),
		#[cfg(any(feature = "with-bifrost-kusama-runtime", feature = "with-bifrost-runtime"))]
		"bifrost-local" | "bifrost-kusama-local" =>
			Box::new(service::chain_spec::bifrost_kusama::local_testnet_config()?),
		#[cfg(any(feature = "with-bifrost-kusama-runtime", feature = "with-bifrost-runtime"))]
		"bifrost-stage" | "bifrost-kusama-stage" =>
			Box::new(service::chain_spec::bifrost_kusama::stage_testnet_config()?),

		#[cfg(any(feature = "with-bifrost-polkadot-runtime", feature = "with-bifrost-runtime"))]
		"bifrost-polkadot" =>
			Box::new(service::chain_spec::bifrost_polkadot::ChainSpec::from_json_bytes(
				&include_bytes!("../../service/res/bifrost-polkadot.json")[..],
			)?),
		#[cfg(any(feature = "with-bifrost-polkadot-runtime", feature = "with-bifrost-runtime"))]
		"bifrost-polkadot-genesis" => Box::new(service::chain_spec::bifrost_polkadot::chainspec_config()),
		#[cfg(any(feature = "with-bifrost-polkadot-runtime", feature = "with-bifrost-runtime"))]
		"bifrost-polkadot-local" =>
			Box::new(service::chain_spec::bifrost_polkadot::local_testnet_config()?),
		#[cfg(feature = "with-bifrost-kusama-runtime")]
		"dev" => Box::new(service::chain_spec::bifrost_kusama::development_config()?),
		#[cfg(feature = "with-bifrost-polkadot-runtime")]
		"bifrost-polkadot-dev" => Box::new(service::chain_spec::bifrost_polkadot::development_config()?),
		path => {
			let path = std::path::PathBuf::from(path);
			if path.to_str().map(|s| s.contains("bifrost-polkadot")) == Some(true) {
				#[cfg(any(
					feature = "with-bifrost-polkadot-runtime",
					feature = "with-bifrost-runtime"
				))]
				{
					Box::new(service::chain_spec::bifrost_polkadot::ChainSpec::from_json_file(
						path,
					)?)
				}
				#[cfg(not(any(
					feature = "with-bifrost-polkadot-runtime",
					feature = "with-bifrost-runtime"
				)))]
				return Err(service::BIFROST_POLKADOT_RUNTIME_NOT_AVAILABLE.into());
			} else if path.to_str().map(|s| s.contains("bifrost")) == Some(true) {
				#[cfg(any(
					feature = "with-bifrost-kusama-runtime",
					feature = "with-bifrost-runtime"
				))]
				{
					Box::new(service::chain_spec::bifrost_kusama::ChainSpec::from_json_file(path)?)
				}
				#[cfg(not(any(
					feature = "with-bifrost-kusama-runtime",
					feature = "with-bifrost-runtime"
				)))]
				return Err(service::BIFROST_KUSAMA_RUNTIME_NOT_AVAILABLE.into());
			} else {
				return Err(service::UNKNOWN_RUNTIME.into());
			}
		},
	})
}

impl SubstrateCli for Cli {
	fn impl_name() -> String {
		"Bifrost Collator".into()
	}

	fn impl_version() -> String {
		env!("SUBSTRATE_CLI_IMPL_VERSION").into()
	}

	fn description() -> String {
		format!(
			"Bifrost collator\n\nThe command-line arguments provided first will be \
		passed to the parachain node, while the arguments provided after -- will be passed \
		to the relaychain node.\n\n\
		{} [parachain-args] -- [relaychain-args]",
			Self::executable_name()
		)
	}

	fn author() -> String {
		env!("CARGO_PKG_AUTHORS").into()
	}

	fn support_url() -> String {
		"https://github.com/bifrost-finance/bifrost/issues/new".into()
	}

	fn copyright_start_year() -> i32 {
		2019
	}

	fn load_spec(&self, id: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
		load_spec(id)
	}

	fn native_runtime_version(spec: &Box<dyn ChainSpec>) -> &'static RuntimeVersion {
		if spec.is_bifrost_kusama() || spec.is_dev() {
			#[cfg(any(feature = "with-bifrost-kusama-runtime", feature = "with-bifrost-runtime"))]
			{
				&bifrost_kusama_runtime::VERSION
			}
			#[cfg(not(any(
				feature = "with-bifrost-kusama-runtime",
				feature = "with-bifrost-runtime"
			)))]
			panic!("{}", service::BIFROST_KUSAMA_RUNTIME_NOT_AVAILABLE);
		} else if spec.is_bifrost_polkadot() {
			#[cfg(any(
				feature = "with-bifrost-polkadot-runtime",
				feature = "with-bifrost-runtime"
			))]
			{
				&bifrost_polkadot_runtime::VERSION
			}
			#[cfg(not(any(
				feature = "with-bifrost-polkadot-runtime",
				feature = "with-bifrost-runtime"
			)))]
			panic!("{}", service::BIFROST_POLKADOT_RUNTIME_NOT_AVAILABLE);
		} else {
			panic!("{}", "unknown runtime!");
		}
	}
}

impl SubstrateCli for RelayChainCli {
	fn impl_name() -> String {
		"Parachain Collator".into()
	}

	fn impl_version() -> String {
		env!("SUBSTRATE_CLI_IMPL_VERSION").into()
	}

	fn description() -> String {
		"Parachain collator\n\nThe command-line arguments provided first will be \
		passed to the parachain node, while the arguments provided after -- will be passed \
		to the relaychain node.\n\n\
		bifrost-collator [parachain-args] -- [relaychain-args]"
			.into()
	}

	fn author() -> String {
		env!("CARGO_PKG_AUTHORS").into()
	}

	fn support_url() -> String {
		"https://github.com/bifrost-finance/bifrost/issues/new".into()
	}

	fn copyright_start_year() -> i32 {
		2019
	}

	fn load_spec(&self, id: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
		polkadot_cli::Cli::from_iter([RelayChainCli::executable_name()].iter()).load_spec(id)
	}

	fn native_runtime_version(chain_spec: &Box<dyn ChainSpec>) -> &'static RuntimeVersion {
		polkadot_cli::Cli::native_runtime_version(chain_spec)
	}
}

fn extract_genesis_wasm(chain_spec: &Box<dyn sc_service::ChainSpec>) -> Result<Vec<u8>> {
	let mut storage = chain_spec.build_storage()?;

	storage
		.top
		.remove(sp_core::storage::well_known_keys::CODE)
		.ok_or_else(|| "Could not find wasm file in genesis state!".into())
}

#[cfg(any(feature = "with-bifrost-kusama-runtime", feature = "with-bifrost-runtime"))]
use service::collator_kusama::{bifrost_kusama_runtime, BifrostExecutor};
#[cfg(any(feature = "with-bifrost-polkadot-runtime", feature = "with-bifrost-runtime"))]
use service::collator_polkadot::bifrost_polkadot_runtime;

macro_rules! with_runtime_or_err {
	($chain_spec:expr, { $( $code:tt )* }) => {
		if $chain_spec.is_bifrost_kusama() || $chain_spec.is_dev() {
			#[cfg(any(feature = "with-bifrost-kusama-runtime",feature = "with-bifrost-runtime"))]
			#[allow(unused_imports)]
			use bifrost_kusama_runtime::{Block, RuntimeApi};
			#[cfg(any(feature = "with-bifrost-kusama-runtime",feature = "with-bifrost-runtime"))]
			#[allow(unused_imports)]
			use BifrostExecutor as Executor;
			#[cfg(any(feature = "with-bifrost-kusama-runtime",feature = "with-bifrost-runtime"))]
			#[allow(unused_imports)]
			use service::collator_kusama::start_node;
			#[cfg(any(feature = "with-bifrost-kusama-runtime",feature = "with-bifrost-runtime"))]
			#[allow(unused_imports)]
			use service::collator_kusama::new_chain_ops;
			#[cfg(any(feature = "with-bifrost-kusama-runtime", feature = "with-bifrost-runtime"))]
			#[allow(unused_imports)]
			use service::collator_kusama::new_partial;

			#[cfg(any(feature = "with-bifrost-kusama-runtime",feature = "with-bifrost-runtime"))]
			$( $code )*

			#[cfg(not(any(feature = "with-bifrost-kusama-runtime",feature = "with-bifrost-runtime")))]
			return Err(service::BIFROST_KUSAMA_RUNTIME_NOT_AVAILABLE.into());
		} else if $chain_spec.is_bifrost_polkadot() {
			#[cfg(any(feature = "with-bifrost-polkadot-runtime", feature = "with-bifrost-runtime"))]
			#[allow(unused_imports)]
			use bifrost_polkadot_runtime::{Block, RuntimeApi};
			#[cfg(any(feature = "with-bifrost-polkadot-runtime", feature = "with-bifrost-runtime"))]
			#[allow(unused_imports)]
			use BifrostPolkadotExecutor as Executor;
			#[cfg(any(feature = "with-bifrost-polkadot-runtime", feature = "with-bifrost-runtime"))]
			#[allow(unused_imports)]
			use service::collator_polkadot::start_node;
			#[cfg(any(feature = "with-bifrost-polkadot-runtime", feature = "with-bifrost-runtime"))]
			#[allow(unused_imports)]
			use service::collator_polkadot::new_chain_ops;
			#[cfg(any(feature = "with-bifrost-polkadot-runtime", feature = "with-bifrost-runtime"))]
			#[allow(unused_imports)]
			use service::collator_polkadot::new_partial;

			#[cfg(any(feature = "with-bifrost-polkadot-runtime", feature = "with-bifrost-runtime"))]
			$( $code )*

			#[cfg(not(any(feature = "with-bifrost-polkadot-runtime", feature = "with-bifrost-runtime")))]
			return Err(service::BIFROST_POLKADOT_RUNTIME_NOT_AVAILABLE.into());
		} else {
			return Err(service::UNKNOWN_RUNTIME.into());
		}
	}
}

fn set_default_ss58_version(spec: &Box<dyn ChainSpec>) {
	use sp_core::crypto::Ss58AddressFormatRegistry;

	let ss58_version = if spec.is_bifrost_kusama() || spec.is_bifrost_polkadot() {
		Ss58AddressFormatRegistry::BifrostAccount
	} else {
		Ss58AddressFormatRegistry::SubstrateAccount
	};

	sp_core::crypto::set_default_ss58_version(ss58_version.into());
}

/// Parse command line arguments into service configuration.
#[allow(unreachable_code)]
pub fn run() -> Result<()> {
	let cli = Cli::from_args();

	match &cli.subcommand {
		None => {
			let runner = cli.create_runner(&cli.run.normalize())?;
			let collator_options = cli.run.collator_options();

			runner.run_node_until_exit(|config| async move {
				let para_id =
					node_service::chain_spec::RelayExtensions::try_get(&*config.chain_spec)
						.map(|e| e.para_id)
						.ok_or("Could not find parachain ID in chain-spec.")?;

				let polkadot_cli = RelayChainCli::new(
					&config,
					[RelayChainCli::executable_name()].iter().chain(cli.relaychain_args.iter()),
				);

				let id = ParaId::from(para_id);

				let parachain_account =
					AccountIdConversion::<polkadot_primitives::v2::AccountId>::into_account(&id);

				let state_version =
					RelayChainCli::native_runtime_version(&config.chain_spec).state_version();

				let block: node_primitives::Block =
					generate_genesis_block(&config.chain_spec, state_version)
						.map_err(|e| format!("{:?}", e))?;
				let genesis_state = format!("0x{:?}", HexDisplay::from(&block.header().encode()));

				let polkadot_config = SubstrateCli::create_configuration(
					&polkadot_cli,
					&polkadot_cli,
					config.tokio_handle.clone(),
				)
				.map_err(|err| format!("Relay chain argument error: {}", err))?;

				info!("Parachain id: {:?}", id);
				info!("Parachain Account: {}", parachain_account);
				info!("Parachain genesis state: {}", genesis_state);
				info!("Is collating: {}", if config.role.is_authority() { "yes" } else { "no" });

				with_runtime_or_err!(config.chain_spec, {
					{
						start_node::<RuntimeApi>(config, polkadot_config, collator_options, id)
							.await
							.map(|r| r.0)
							.map_err(Into::into)
					}
				})
			})
		},
		Some(Subcommand::Inspect(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;
			set_default_ss58_version(chain_spec);

			with_runtime_or_err!(chain_spec, {
				return runner.sync_run(|config| cmd.run::<Block, RuntimeApi, Executor>(config));
			})
		},
		Some(Subcommand::Benchmark(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;
			set_default_ss58_version(chain_spec);

			// Switch on the concrete benchmark sub-command-
			match cmd {
				BenchmarkCmd::Pallet(cmd) =>
					if cfg!(feature = "runtime-benchmarks") {
						with_runtime_or_err!(chain_spec, {
							return runner.sync_run(|config| cmd.run::<Block, Executor>(config));
						})
					} else {
						Err("Benchmarking wasn't enabled when building the node. \
						You can enable it with `--features runtime-benchmarks`."
							.into())
					},
				BenchmarkCmd::Block(cmd) => runner.sync_run(|config| {
					with_runtime_or_err!(config.chain_spec, {
						{
							let partials = new_partial::<RuntimeApi>(&config, false)?;
							cmd.run(partials.client)
						}
					})
				}),
				BenchmarkCmd::Storage(cmd) => runner.sync_run(|config| {
					with_runtime_or_err!(config.chain_spec, {
						{
							let partials = new_partial::<RuntimeApi>(&config, false)?;
							let db = partials.backend.expose_db();
							let storage = partials.backend.expose_storage();
							cmd.run(config, partials.client.clone(), db, storage)
						}
					})
				}),
				BenchmarkCmd::Overhead(_) => Err("Unsupported benchmarking command".into()),
			}
		},
		Some(Subcommand::Key(cmd)) => cmd.run(&cli),
		Some(Subcommand::Sign(cmd)) => cmd.run(),
		Some(Subcommand::Verify(cmd)) => cmd.run(),
		Some(Subcommand::Vanity(cmd)) => cmd.run(),
		Some(Subcommand::BuildSpec(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| cmd.run(config.chain_spec, config.network))
		},
		Some(Subcommand::CheckBlock(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;

			set_default_ss58_version(chain_spec);

			with_runtime_or_err!(chain_spec, {
				return runner.async_run(|mut config| {
					let (client, _, import_queue, task_manager) = new_chain_ops(&mut config)?;
					Ok((cmd.run(client, import_queue), task_manager))
				});
			})
		},
		Some(Subcommand::ExportBlocks(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;

			set_default_ss58_version(chain_spec);

			with_runtime_or_err!(chain_spec, {
				return runner.async_run(|mut config| {
					let (client, _, _, task_manager) = new_chain_ops(&mut config)?;
					Ok((cmd.run(client, config.database), task_manager))
				});
			})
		},
		Some(Subcommand::ExportState(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;

			set_default_ss58_version(chain_spec);

			with_runtime_or_err!(chain_spec, {
				return runner.async_run(|mut config| {
					let (client, _, _, task_manager) = new_chain_ops(&mut config)?;
					Ok((cmd.run(client, config.chain_spec), task_manager))
				});
			})
		},
		Some(Subcommand::ImportBlocks(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;

			set_default_ss58_version(chain_spec);

			with_runtime_or_err!(chain_spec, {
				return runner.async_run(|mut config| {
					let (client, _, import_queue, task_manager) = new_chain_ops(&mut config)?;
					Ok((cmd.run(client, import_queue), task_manager))
				});
			})
		},
		Some(Subcommand::PurgeChain(cmd)) => {
			let runner = cli.create_runner(cmd)?;

			runner.sync_run(|config| {
				let polkadot_cli = RelayChainCli::new(
					&config,
					[RelayChainCli::executable_name()].iter().chain(cli.relaychain_args.iter()),
				);

				let polkadot_config = SubstrateCli::create_configuration(
					&polkadot_cli,
					&polkadot_cli,
					config.tokio_handle.clone(),
				)
				.map_err(|err| format!("Relay chain argument error: {}", err))?;

				cmd.run(config, polkadot_config)
			})
		},
		Some(Subcommand::Revert(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;

			set_default_ss58_version(chain_spec);
			with_runtime_or_err!(chain_spec, {
				return runner.async_run(|mut config| {
					let (client, backend, _, task_manager) = new_chain_ops(&mut config)?;
					Ok((cmd.run(client, backend, None), task_manager))
				});
			})
		},
		Some(Subcommand::ExportGenesisState(params)) => {
			let mut builder = sc_cli::LoggerBuilder::new("");
			builder.with_profiling(sc_tracing::TracingReceiver::Log, "");
			let _ = builder.init();

			let chain_spec = cli.load_spec(&params.chain.clone().unwrap_or_default())?;
			let state_version = Cli::native_runtime_version(&chain_spec).state_version();
			let output_buf = with_runtime_or_err!(chain_spec, {
				{
					let block: Block = generate_genesis_block(&chain_spec, state_version)
						.map_err(|e| format!("{:?}", e))?;
					let raw_header = block.header().encode();
					let buf = if params.raw {
						raw_header
					} else {
						format!("0x{:?}", HexDisplay::from(&block.header().encode())).into_bytes()
					};
					buf
				}
			});
			if let Some(output) = &params.output {
				std::fs::write(output, output_buf)?;
			} else {
				std::io::stdout().write_all(&output_buf)?;
			}

			Ok(())
		},
		Some(Subcommand::ExportGenesisWasm(params)) => {
			let mut builder = sc_cli::LoggerBuilder::new("");
			builder.with_profiling(sc_tracing::TracingReceiver::Log, "");
			let _ = builder.init();

			let raw_wasm_blob =
				extract_genesis_wasm(&cli.load_spec(&params.chain.clone().unwrap_or_default())?)?;
			let output_buf = if params.raw {
				raw_wasm_blob
			} else {
				format!("0x{:?}", HexDisplay::from(&raw_wasm_blob)).into_bytes()
			};

			if let Some(output) = &params.output {
				std::fs::write(output, output_buf)?;
			} else {
				std::io::stdout().write_all(&output_buf)?;
			}

			Ok(())
		},
		#[cfg(feature = "try-runtime")]
		Some(Subcommand::TryRuntime(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;

			set_default_ss58_version(chain_spec);

			with_runtime_or_err!(chain_spec, {
				return runner.async_run(|config| {
					let registry = config.prometheus_config.as_ref().map(|cfg| &cfg.registry);
					let task_manager =
						sc_service::TaskManager::new(config.tokio_handle.clone(), registry)
							.map_err(|e| {
								sc_cli::Error::Service(sc_service::Error::Prometheus(e))
							})?;
					Ok((cmd.run::<Block, Executor>(config), task_manager))
				});
			})
		},
	}
}

impl DefaultConfigurationValues for RelayChainCli {
	fn p2p_listen_port() -> u16 {
		30334
	}

	fn rpc_ws_listen_port() -> u16 {
		9945
	}

	fn rpc_http_listen_port() -> u16 {
		9934
	}

	fn prometheus_listen_port() -> u16 {
		9616
	}
}

impl CliConfiguration<Self> for RelayChainCli {
	fn shared_params(&self) -> &SharedParams {
		self.base.base.shared_params()
	}

	fn import_params(&self) -> Option<&ImportParams> {
		self.base.base.import_params()
	}

	fn network_params(&self) -> Option<&NetworkParams> {
		self.base.base.network_params()
	}

	fn keystore_params(&self) -> Option<&KeystoreParams> {
		self.base.base.keystore_params()
	}

	fn base_path(&self) -> Result<Option<BasePath>> {
		Ok(self
			.shared_params()
			.base_path()
			.or_else(|| self.base_path.clone().map(Into::into)))
	}

	fn rpc_http(&self, default_listen_port: u16) -> Result<Option<SocketAddr>> {
		self.base.base.rpc_http(default_listen_port)
	}

	fn rpc_ipc(&self) -> Result<Option<String>> {
		self.base.base.rpc_ipc()
	}

	fn rpc_ws(&self, default_listen_port: u16) -> Result<Option<SocketAddr>> {
		self.base.base.rpc_ws(default_listen_port)
	}

	fn prometheus_config(
		&self,
		default_listen_port: u16,
		chain_spec: &Box<dyn ChainSpec>,
	) -> Result<Option<PrometheusConfig>> {
		self.base.base.prometheus_config(default_listen_port, chain_spec)
	}

	fn init<F>(
		&self,
		_support_url: &String,
		_impl_version: &String,
		_logger_hook: F,
		_config: &sc_service::Configuration,
	) -> Result<()>
	where
		F: FnOnce(&mut sc_cli::LoggerBuilder, &sc_service::Configuration),
	{
		unreachable!("PolkadotCli is never initialized; qed");
	}

	fn chain_id(&self, is_dev: bool) -> Result<String> {
		let chain_id = self.base.base.chain_id(is_dev)?;

		Ok(if chain_id.is_empty() { self.chain_id.clone().unwrap_or_default() } else { chain_id })
	}

	fn role(&self, is_dev: bool) -> Result<sc_service::Role> {
		self.base.base.role(is_dev)
	}

	fn transaction_pool(&self) -> Result<sc_service::config::TransactionPoolOptions> {
		self.base.base.transaction_pool()
	}

	fn state_cache_child_ratio(&self) -> Result<Option<usize>> {
		self.base.base.state_cache_child_ratio()
	}

	fn rpc_methods(&self) -> Result<sc_service::config::RpcMethods> {
		self.base.base.rpc_methods()
	}

	fn rpc_ws_max_connections(&self) -> Result<Option<usize>> {
		self.base.base.rpc_ws_max_connections()
	}

	fn rpc_cors(&self, is_dev: bool) -> Result<Option<Vec<String>>> {
		self.base.base.rpc_cors(is_dev)
	}

	fn default_heap_pages(&self) -> Result<Option<u64>> {
		self.base.base.default_heap_pages()
	}

	fn force_authoring(&self) -> Result<bool> {
		self.base.base.force_authoring()
	}

	fn disable_grandpa(&self) -> Result<bool> {
		self.base.base.disable_grandpa()
	}

	fn max_runtime_instances(&self) -> Result<Option<usize>> {
		self.base.base.max_runtime_instances()
	}

	fn announce_block(&self) -> Result<bool> {
		self.base.base.announce_block()
	}

	fn telemetry_endpoints(
		&self,
		chain_spec: &Box<dyn ChainSpec>,
	) -> Result<Option<sc_telemetry::TelemetryEndpoints>> {
		self.base.base.telemetry_endpoints(chain_spec)
	}

	fn node_name(&self) -> Result<String> {
		self.base.base.node_name()
	}
}
