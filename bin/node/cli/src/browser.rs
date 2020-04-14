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

use crate::chain_spec::ChainSpec;
use log::info;
use wasm_bindgen::prelude::*;
use sc_service::Configuration;
use browser_utils::{
	Client,
	browser_configuration, set_console_error_panic_hook, init_console_log,
};
use std::str::FromStr;

/// Starts the client.
#[wasm_bindgen]
pub async fn start_client(chain_spec: String, log_level: String) -> Result<Client, JsValue> {
	start_inner(chain_spec, log_level)
		.await
		.map_err(|err| JsValue::from_str(&err.to_string()))
}

async fn start_inner(chain_spec: String, log_level: String) -> Result<Client, Box<dyn std::error::Error>> {
	set_console_error_panic_hook();
	init_console_log(log::Level::from_str(&log_level)?)?;
	let chain_spec = ChainSpec::from_json_bytes(chain_spec.as_bytes().to_vec())
		.map_err(|e| format!("{:?}", e))?;

	let config = browser_configuration(chain_spec).await?;

	info!("Bifrost browser node");
	info!("✌️  version {}", config.impl_version);
	info!("❤️  by Liebi Technologies, 2019-2020");
	info!("📋  Chain specification: {}", config.chain_spec.name());
	info!("🏷  Node name: {}", config.network.node_name);
	info!("👤 Role: {:?}", config.role);

	// Create the service. This is the most heavy initialization step.
	let service = crate::service::new_light(config)
		.map_err(|e| format!("{:?}", e))?;

	Ok(browser_utils::start_client(service))
}
