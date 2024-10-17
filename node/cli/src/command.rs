// This file is part of Bifrost.

// Copyright (C) Liebi Technologies PTE. LTD.
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

use bifrost_service::{self as service, IdentifyVariant};
use cumulus_client_service::storage_proof_size::HostFunctions as ReclaimHostFunctions;
use cumulus_primitives_core::ParaId;
use frame_benchmarking_cli::{BenchmarkCmd, SUBSTRATE_REFERENCE_HARDWARE};
use log::info;
use sc_cli::{
	ChainSpec, CliConfiguration, DefaultConfigurationValues, ImportParams, KeystoreParams,
	NetworkParams, Result, SharedParams, SubstrateCli,
};
use sc_service::config::{BasePath, PrometheusConfig};
use sp_runtime::traits::{AccountIdConversion, HashingFor};

use crate::{Cli, RelayChainCli, Subcommand};

fn get_exec_name() -> Option<String> {
	std::env::current_exe()
		.ok()
		.and_then(|pb| pb.file_name().map(|s| s.to_os_string()))
		.and_then(|s| s.into_string().ok())
}

fn load_spec(id: &str) -> std::result::Result<Box<dyn ChainSpec>, String> {
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
	#[allow(unreachable_code)]
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
			Box::new(service::chain_spec::bifrost_kusama::local_testnet_config()),
		#[cfg(any(feature = "with-bifrost-polkadot-runtime", feature = "with-bifrost-runtime"))]
		"bifrost-polkadot" =>
			Box::new(service::chain_spec::bifrost_polkadot::ChainSpec::from_json_bytes(
				&include_bytes!("../../service/res/bifrost-polkadot.json")[..],
			)?),
		#[cfg(any(feature = "with-bifrost-polkadot-runtime", feature = "with-bifrost-runtime"))]
		"bifrost-polkadot-genesis" => Box::new(service::chain_spec::bifrost_polkadot::chainspec_config()),
		#[cfg(any(feature = "with-bifrost-polkadot-runtime", feature = "with-bifrost-runtime"))]
		"bifrost-polkadot-local" =>
			Box::new(service::chain_spec::bifrost_polkadot::local_testnet_config()),
		#[cfg(any(feature = "with-bifrost-polkadot-runtime", feature = "with-bifrost-runtime"))]
		"bifrost-paseo" => Box::new(service::chain_spec::bifrost_polkadot::paseo_config()),
		#[cfg(any(feature = "with-bifrost-polkadot-runtime", feature = "with-bifrost-runtime"))]
		"bifrost-polkadot-dev" => Box::new(service::chain_spec::bifrost_polkadot::dev_config()),
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
		"https://github.com/bifrost-io/bifrost/issues/new".into()
	}

	fn copyright_start_year() -> i32 {
		2019
	}

	fn load_spec(&self, id: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
		load_spec(id)
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
		"https://github.com/bifrost-io/bifrost/issues/new".into()
	}

	fn copyright_start_year() -> i32 {
		2019
	}

	fn load_spec(&self, id: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
		polkadot_cli::Cli::from_iter([RelayChainCli::executable_name()].iter()).load_spec(id)
	}
}

macro_rules! with_runtime_or_err {
	($chain_spec:expr, { $( $code:tt )* }) => {
		if $chain_spec.is_bifrost_kusama() {
			#[cfg(any(feature = "with-bifrost-kusama-runtime",feature = "with-bifrost-runtime"))]
			#[allow(unused_imports)]
			use service::collator_kusama::{bifrost_kusama_runtime::{Block, RuntimeApi}, start_node,new_partial};

			#[cfg(any(feature = "with-bifrost-kusama-runtime",feature = "with-bifrost-runtime"))]
			$( $code )*

			#[cfg(not(any(feature = "with-bifrost-kusama-runtime",feature = "with-bifrost-runtime")))]
			return Err(service::BIFROST_KUSAMA_RUNTIME_NOT_AVAILABLE.into());
		} else if $chain_spec.is_bifrost_polkadot() || $chain_spec.is_dev() {
			#[cfg(any(feature = "with-bifrost-polkadot-runtime", feature = "with-bifrost-runtime"))]
			#[allow(unused_imports)]
			use service::collator_polkadot::{bifrost_polkadot_runtime::{Block, RuntimeApi}, start_node,new_partial};

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
		Some(Subcommand::BuildSpec(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| cmd.run(config.chain_spec, config.network))
		},
		Some(Subcommand::CheckBlock(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;

			set_default_ss58_version(chain_spec);

			with_runtime_or_err!(chain_spec, {
				return runner.async_run(|config| {
					let components = new_partial(&config, false)?;
					Ok((
						cmd.run(components.client, components.import_queue),
						components.task_manager,
					))
				});
			})
		},
		Some(Subcommand::ExportBlocks(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;

			set_default_ss58_version(chain_spec);

			with_runtime_or_err!(chain_spec, {
				return runner.async_run(|config| {
					let components = new_partial(&config, false)?;
					Ok((cmd.run(components.client, config.database), components.task_manager))
				});
			})
		},
		Some(Subcommand::ExportState(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;

			set_default_ss58_version(chain_spec);

			with_runtime_or_err!(chain_spec, {
				return runner.async_run(|config| {
					let components = new_partial(&config, false)?;
					Ok((cmd.run(components.client, config.chain_spec), components.task_manager))
				});
			})
		},
		Some(Subcommand::ExportGenesisHead(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;

			with_runtime_or_err!(chain_spec, {
				return runner.sync_run(|config| {
					let partials = new_partial(&config, false)?;
					cmd.run(partials.client)
				});
			})
		},
		Some(Subcommand::ExportGenesisWasm(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|_config| {
				let spec = cli.load_spec(&cmd.shared_params.chain.clone().unwrap_or_default())?;
				cmd.run(&*spec)
			})
		},
		Some(Subcommand::Inspect(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;
			set_default_ss58_version(chain_spec);

			with_runtime_or_err!(chain_spec, {
				return runner.sync_run(|config| cmd.run::<Block, RuntimeApi>(config));
			})
		},
		Some(Subcommand::ImportBlocks(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;

			set_default_ss58_version(chain_spec);
			with_runtime_or_err!(chain_spec, {
				return runner.async_run(|config| {
					let components = new_partial(&config, false)?;
					Ok((
						cmd.run(components.client, components.import_queue),
						components.task_manager,
					))
				});
			})
		},
		Some(Subcommand::PurgeChain(cmd)) => {
			let runner = cli.create_runner(cmd)?;

			runner.sync_run(|config| {
				let polkadot_cli = RelayChainCli::new(
					&config,
					[RelayChainCli::executable_name()].iter().chain(cli.relay_chain_args.iter()),
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
				return runner.async_run(|config| {
					let components = new_partial(&config, false)?;
					Ok((
						cmd.run(components.client, components.backend, None),
						components.task_manager,
					))
				});
			})
		},
		Some(Subcommand::Key(cmd)) => cmd.run(&cli),
		Some(Subcommand::Sign(cmd)) => cmd.run(),
		Some(Subcommand::Verify(cmd)) => cmd.run(),
		Some(Subcommand::Vanity(cmd)) => cmd.run(),
		Some(Subcommand::Benchmark(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;
			set_default_ss58_version(chain_spec);

			// Switch on the concrete benchmark sub-command-
			match cmd {
				BenchmarkCmd::Pallet(cmd) =>
					if cfg!(feature = "runtime-benchmarks") {
						with_runtime_or_err!(chain_spec, {
							return runner.sync_run(|config| {
								cmd.run_with_spec::<HashingFor<Block>, ReclaimHostFunctions>(Some(
									config.chain_spec,
								))
							});
						})
					} else {
						Err("Benchmarking wasn't enabled when building the node. \
						You can enable it with `--features runtime-benchmarks`."
							.into())
					},
				BenchmarkCmd::Block(cmd) => runner.sync_run(|config| {
					with_runtime_or_err!(config.chain_spec, {
						{
							let partials = new_partial(&config, false)?;
							cmd.run(partials.client)
						}
					})
				}),
				#[cfg(not(feature = "runtime-benchmarks"))]
				BenchmarkCmd::Storage(_) =>
					return Err(sc_cli::Error::Input(
						"Compile with --features=runtime-benchmarks \
						to enable storage benchmarks."
							.into(),
					)
					.into()),
				#[cfg(feature = "runtime-benchmarks")]
				BenchmarkCmd::Storage(cmd) => runner.sync_run(|config| {
					with_runtime_or_err!(config.chain_spec, {
						{
							let partials = new_partial(&config, false)?;
							let db = partials.backend.expose_db();
							let storage = partials.backend.expose_storage();
							cmd.run(config, partials.client.clone(), db, storage)
						}
					})
				}),
				BenchmarkCmd::Overhead(_) => Err("Unsupported benchmarking command".into()),
				BenchmarkCmd::Machine(cmd) =>
					runner.sync_run(|config| cmd.run(&config, SUBSTRATE_REFERENCE_HARDWARE.clone())),
				// NOTE: this allows the Client to leniently implement
				// new benchmark commands without requiring a companion MR.
				#[allow(unreachable_patterns)]
				_ => Err("Benchmarking sub-command unsupported".into()),
			}
		},
		None => {
			let runner = cli.create_runner(&cli.run.normalize())?;
			let collator_options = cli.run.collator_options();

			runner.run_node_until_exit(|config| async move {
				let hwbench = (!cli.no_hardware_benchmarks)
					.then_some(config.database.path().map(|database_path| {
						let _ = std::fs::create_dir_all(database_path);
						sc_sysinfo::gather_hwbench(Some(database_path))
					}))
					.flatten();

				let para_id =
					bifrost_service::chain_spec::RelayExtensions::try_get(&config.chain_spec)
						.map(|e| e.para_id)
						.ok_or("Could not find parachain ID in chain-spec.")?;

				let polkadot_cli = RelayChainCli::new(
					&config,
					[RelayChainCli::executable_name()].iter().chain(cli.relay_chain_args.iter()),
				);

				let id = ParaId::from(para_id);

				let parachain_account =
					AccountIdConversion::<polkadot_primitives::AccountId>::into_account_truncating(
						&id,
					);

				let polkadot_config = SubstrateCli::create_configuration(
					&polkadot_cli,
					&polkadot_cli,
					config.tokio_handle.clone(),
				)
				.map_err(|err| format!("Relay chain argument error: {}", err))?;

				info!("Parachain id: {:?}", id);
				info!("Parachain Account: {}", parachain_account);
				info!("Is collating: {}", if config.role.is_authority() { "yes" } else { "no" });
				info!("Is dev modle: {}", if config.chain_spec.is_dev() { "yes" } else { "no" });

				with_runtime_or_err!(config.chain_spec, {
					{
						start_node::<sc_network::NetworkWorker<_, _>>(
							config,
							polkadot_config,
							cli.eth_config,
							collator_options,
							id,
							hwbench,
						)
						.await
						.map(|r| r.0)
						.map_err(Into::into)
					}
				})
			})
		},
	}
}

impl DefaultConfigurationValues for RelayChainCli {
	fn p2p_listen_port() -> u16 {
		30334
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
			.base_path()?
			.or_else(|| self.base_path.clone().map(Into::into)))
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

	fn transaction_pool(&self, is_dev: bool) -> Result<sc_service::config::TransactionPoolOptions> {
		self.base.base.transaction_pool(is_dev)
	}

	fn trie_cache_maximum_size(&self) -> Result<Option<usize>> {
		self.base.base.trie_cache_maximum_size()
	}

	fn rpc_methods(&self) -> Result<sc_service::config::RpcMethods> {
		self.base.base.rpc_methods()
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
