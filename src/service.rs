// Copyright 2019 Liebi Technologies.
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

//! Service and ServiceFactory implementation. Specialized wrapper over Bifrost service.

#![warn(unused_extern_crates)]

use std::sync::Arc;
use log::info;
use transaction_pool::{self, txpool::{Pool as TransactionPool}};
use node_runtime::{self, GenesisConfig, opaque::Block, RuntimeApi, WASM_BINARY};
use substrate_service::{
	FactoryFullConfiguration, LightComponents, FullComponents, FullBackend,
	FullClient, LightClient, LightBackend, FullExecutor, LightExecutor,
	error::{Error as ServiceError},
};
use basic_authorship::ProposerFactory;
use consensus::{import_queue, start_aura, AuraImportQueue, SlotDuration};
use futures::prelude::*;
use substrate_client::{self as client, LongestChain};
use primitives::{ed25519::Pair, Pair as PairT};
use inherents::InherentDataProviders;
use network::{config::DummyFinalityProofRequestBuilder, construct_simple_protocol};
use substrate_executor::native_executor_instance;
use substrate_service::construct_service_factory;

pub use substrate_executor::NativeExecutor;
// Our native executor instance.
native_executor_instance!(
	pub Executor,
	node_runtime::api::dispatch,
	node_runtime::native_version,
	WASM_BINARY
);

#[derive(Default)]
pub struct NodeConfig {
	inherent_data_providers: InherentDataProviders,
}

construct_simple_protocol! {
	/// Demo protocol attachment for substrate.
	pub struct NodeProtocol where Block = Block { }
}

construct_service_factory! {
	struct Factory {
		Block = Block,
		ConsensusPair = Pair,
		FinalityPair = Pair,
		RuntimeApi = RuntimeApi,
		NetworkProtocol = NodeProtocol { |config| Ok(NodeProtocol::new()) },
		RuntimeDispatch = Executor,
		FullTransactionPoolApi = transaction_pool::ChainApi<
			client::Client<FullBackend<Self>, FullExecutor<Self>, Block, RuntimeApi>,
			Block
		> {
			|config, client| Ok(TransactionPool::new(config, transaction_pool::ChainApi::new(client)))
		},
		LightTransactionPoolApi = transaction_pool::ChainApi<
			client::Client<LightBackend<Self>, LightExecutor<Self>, Block, RuntimeApi>,
			Block
		> {
			|config, client| Ok(TransactionPool::new(config, transaction_pool::ChainApi::new(client)))
		},
		Genesis = GenesisConfig,
		Configuration = NodeConfig,
		FullService = FullComponents<Self>
			{ |config: FactoryFullConfiguration<Self>|
				FullComponents::<Factory>::new(config)
			},
		AuthoritySetup = {
			|service: Self::FullService| {
				if let Some(key) = service.authority_key() {
					info!("Using authority key {}", key.public());
					let proposer = Arc::new(ProposerFactory {
						client: service.client(),
						transaction_pool: service.transaction_pool(),
					});
					let client = service.client();
					let select_chain = service.select_chain()
						.ok_or_else(|| ServiceError::SelectChainRequired)?;
					let aura = start_aura(
						SlotDuration::get_or_compute(&*client)?,
						Arc::new(key),
						client.clone(),
						select_chain,
						client,
						proposer,
						service.network(),
						service.config.custom.inherent_data_providers.clone(),
						service.config.force_authoring,
					)?;
					service.spawn_task(Box::new(aura.select(service.on_exit()).then(|_| Ok(()))));
				}

				Ok(service)
			}
		},
		LightService = LightComponents<Self>
			{ |config| <LightComponents<Factory>>::new(config) },
		FullImportQueue = AuraImportQueue<
			Self::Block,
		>
			{ |config: &mut FactoryFullConfiguration<Self> , client: Arc<FullClient<Self>>, _select_chain: Self::SelectChain| {
					import_queue::<_, _, Pair>(
						SlotDuration::get_or_compute(&*client)?,
						Box::new(client.clone()),
						None,
						None,
						client,
						config.custom.inherent_data_providers.clone(),
					).map_err(Into::into)
				}
			},
		LightImportQueue = AuraImportQueue<
			Self::Block,
		>
			{ |config: &mut FactoryFullConfiguration<Self>, client: Arc<LightClient<Self>>| {
					let fprb = Box::new(DummyFinalityProofRequestBuilder::default()) as Box<_>;
					import_queue::<_, _, Pair>(
						SlotDuration::get_or_compute(&*client)?,
						Box::new(client.clone()),
						None,
						None,
						client,
						config.custom.inherent_data_providers.clone(),
					).map(|q| (q, fprb)).map_err(Into::into)
				}
			},
		SelectChain = LongestChain<FullBackend<Self>, Self::Block>
			{ |config: &FactoryFullConfiguration<Self>, client: Arc<FullClient<Self>>| {
				#[allow(deprecated)]
				Ok(LongestChain::new(client.backend().clone()))
			}
		},
		FinalityProofProvider = { |_client: Arc<FullClient<Self>>| {
			Ok(None)
		}},
	}
}
