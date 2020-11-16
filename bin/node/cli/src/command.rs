// Copyright 2019-2020 Liebi Technologies.
// This file is part of Bifrost.

// Bifrost is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Bifrost is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Bifrost.  If not, see <http://www.gnu.org/licenses/>.

use std::io::Write;

use codec::Encode;
use sc_cli::{ChainSpec, Result, Role, RuntimeVersion, SubstrateCli};
use sp_core::hexdisplay::HexDisplay;
use sp_runtime::traits::{Block as BlockT, Hash as HashT, Header as HeaderT, Zero};

use node_primitives::Block;
use node_service::{self as service, IdentifyVariant};

use crate::{Cli, Subcommand};

fn get_exec_name() -> Option<String> {
	std::env::current_exe()
		.ok()
		.and_then(|pb| pb.file_name().map(|s| s.to_os_string()))
		.and_then(|s| s.into_string().ok())
}

fn load_spec(
	id: &str,
) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
	let id = if id == "" {
		let n = get_exec_name().unwrap_or_default();
		["asgard", "bifrost"].iter()
			.cloned()
			.find(|&chain| n.starts_with(chain))
			.unwrap_or("bifrost")
	} else { id };
	Ok(match id {
		"asgard" => Box::new(service::chain_spec::asgard::chainspec_config()),
		"asgard-dev" => Box::new(service::chain_spec::asgard::development_config()?),
		"asgard-local" => Box::new(service::chain_spec::asgard::local_testnet_config()?),
		"asgard-staging" => Box::new(service::chain_spec::asgard::staging_testnet_config()),
		"bifrost" | "" => Box::new(service::chain_spec::bifrost::chainspec_config()),
		"bifrost-dev" | "dev" => Box::new(service::chain_spec::bifrost::development_config()?),
		"bifrost-local" | "local" => Box::new(service::chain_spec::bifrost::local_testnet_config()?),
		"bifrost-staging" | "staging" => Box::new(service::chain_spec::bifrost::staging_testnet_config()),
		"rococo" => Box::new(service::chain_spec::rococo::chainspec_config()),
		"rococo-dev" => Box::new(service::chain_spec::rococo::development_config()?),
		"rococo-local" => Box::new(service::chain_spec::rococo::local_testnet_config()?),
		"rococo-staging" => Box::new(service::chain_spec::rococo::staging_testnet_config()),
		path => {
			let path = std::path::PathBuf::from(path);
			Box::new(service::chain_spec::bifrost::ChainSpec::from_json_file(path)?)
		}
	})
}

impl SubstrateCli for Cli {
	fn impl_name() -> String {
		"Bifrost".into()
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
		"https://github.com/bifrost-finance/bifrost/issues/new".into()
	}

	fn copyright_start_year() -> i32 {
		2019
	}

	fn load_spec(&self, id: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
		load_spec(id)
	}

	fn native_runtime_version(spec: &Box<dyn ChainSpec>) -> &'static RuntimeVersion {
		if spec.is_asgard() {
			&service::asgard_runtime::VERSION
		} else if spec.is_bifrost() {
			&service::bifrost_runtime::VERSION
		} else if spec.is_rococo() {
			&service::rococo_runtime::VERSION
		} else {
			&service::bifrost_runtime::VERSION
		}
	}
}

fn set_default_ss58_version(spec: &Box<dyn ChainSpec>) {
	use sp_core::crypto::Ss58AddressFormat;

	let ss58_version = if spec.is_asgard() {
		Ss58AddressFormat::BifrostAccount
	} else if spec.is_bifrost() {
		Ss58AddressFormat::BifrostAccount
	} else {
		Ss58AddressFormat::BifrostAccount
	};

	sp_core::crypto::set_default_ss58_version(ss58_version);
}

fn extract_genesis_wasm(chain_spec: &Box<dyn sc_service::ChainSpec>) -> Result<Vec<u8>> {
	let mut storage = chain_spec.build_storage()?;

	storage
		.top
		.remove(sp_core::storage::well_known_keys::CODE)
		.ok_or_else(|| "Could not find wasm file in genesis state!".into())
}

/// Generate the genesis state for a given ChainSpec.
fn generate_genesis_block<Block: BlockT>(
	chain_spec: &Box<dyn ChainSpec>,
) -> Result<Block> {
	let storage = chain_spec.build_storage()?;

	let child_roots = storage.children_default.iter().map(|(sk, child_content)| {
		let state_root = <<<Block as BlockT>::Header as HeaderT>::Hashing as HashT>::trie_root(
			child_content.data.clone().into_iter().collect(),
		);
		(sk.clone(), state_root.encode())
	});
	let state_root = <<<Block as BlockT>::Header as HeaderT>::Hashing as HashT>::trie_root(
		storage.top.clone().into_iter().chain(child_roots).collect(),
	);

	let extrinsics_root =
		<<<Block as BlockT>::Header as HeaderT>::Hashing as HashT>::trie_root(Vec::new());

	Ok(Block::new(
		<<Block as BlockT>::Header as HeaderT>::new(
			Zero::zero(),
			extrinsics_root,
			state_root,
			Default::default(),
			Default::default(),
		),
		Default::default(),
	))
}

/// Parse command line arguments into service configuration.
pub fn run() -> Result<()> {
	let cli = Cli::from_args();

	match &cli.subcommand {
		None => {
			let runner = cli.create_runner(&cli.run)?;
			runner.run_node_until_exit(|config| async move {
				match config.role {
					Role::Light => service::build_light(config),
					_ => service::build_full(config),
				}
			})
		}
		Some(Subcommand::Inspect(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;

			set_default_ss58_version(chain_spec);

			runner.sync_run(|config| {
				cmd.run::<
					service::bifrost_runtime::Block,
					service::bifrost_runtime::RuntimeApi,
					service::BifrostExecutor
				>(config)
			})
		}
		Some(Subcommand::Benchmark(cmd)) => {
			if cfg!(feature = "runtime-benchmarks") {
				let runner = cli.create_runner(cmd)?;
				let chain_spec = &runner.config().chain_spec;

				set_default_ss58_version(chain_spec);

				runner.sync_run(|config| {
					cmd.run::<service::bifrost_runtime::Block, service::BifrostExecutor>(config)
				})
			} else {
				Err("Benchmarking wasn't enabled when building the node. \
				You can enable it with `--features runtime-benchmarks`.".into())
			}
		}
		Some(Subcommand::Key(cmd)) => cmd.run(),
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

			runner.async_run(|mut config| {
				let (client, _, import_queue, task_manager) = service::new_chain_ops(&mut config)?;
				Ok((cmd.run(client, import_queue), task_manager))
			})
		},
		Some(Subcommand::ExportBlocks(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;

			set_default_ss58_version(chain_spec);

			runner.async_run(|mut config| {
				let (client, _, _, task_manager) = service::new_chain_ops(&mut config)?;
				Ok((cmd.run(client, config.database), task_manager))
			})
		},
		Some(Subcommand::ExportState(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;

			set_default_ss58_version(chain_spec);

			runner.async_run(|mut config| {
				let (client, _, _, task_manager) = service::new_chain_ops(&mut config)?;
				Ok((cmd.run(client, config.chain_spec), task_manager))
			})
		},
		Some(Subcommand::ImportBlocks(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;

			set_default_ss58_version(chain_spec);

			runner.async_run(|mut config| {
				let (client, _, import_queue, task_manager) = service::new_chain_ops(&mut config)?;
				Ok((cmd.run(client, import_queue), task_manager))
			})
		},
		Some(Subcommand::PurgeChain(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| cmd.run(config.database))
		},
		Some(Subcommand::Revert(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;

			set_default_ss58_version(chain_spec);

			runner.async_run(|mut config| {
				let (client, backend, _, task_manager) = service::new_chain_ops(&mut config)?;
				Ok((cmd.run(client, backend), task_manager))
			})
		}
		Some(Subcommand::ExportGenesisState(params)) => {
			sc_cli::init_logger("", sc_tracing::TracingReceiver::Log, None)?;

			let block: Block = generate_genesis_block(&load_spec(
				&params.chain.clone().unwrap_or_default(),
			)?)?;
			let raw_header = block.header().encode();
			let output_buf = if params.raw {
				raw_header
			} else {
				format!("0x{:?}", HexDisplay::from(&block.header().encode())).into_bytes()
			};

			if let Some(output) = &params.output {
				std::fs::write(output, output_buf)?;
			} else {
				std::io::stdout().write_all(&output_buf)?;
			}

			Ok(())
		}
		Some(Subcommand::ExportGenesisWasm(params)) => {
			sc_cli::init_logger("", sc_tracing::TracingReceiver::Log, None)?;

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
	}
}
