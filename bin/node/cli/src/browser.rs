use crate::ChainSpec;
use log::info;
use wasm_bindgen::prelude::*;
use sc_service::Configuration;
use browser_utils::{
    Transport, Client,
    browser_configuration, set_console_error_panic_hook, init_console_log,
};

/// Starts the client.
#[wasm_bindgen]
pub async fn start_client(wasm_ext: Transport) -> Result<Client, JsValue> {
    start_inner(wasm_ext)
        .await
        .map_err(|err| JsValue::from_str(&err.to_string()))
}

async fn start_inner(wasm_ext: Transport) -> Result<Client, Box<dyn std::error::Error>> {
    set_console_error_panic_hook();
    init_console_log(log::Level::Info)?;

    let chain_spec = ChainSpec::FlamingFir.load()
        .map_err(|e| format!("{:?}", e))?;

    let config: Configuration<_, _> = browser_configuration(wasm_ext, chain_spec)
        .await?;

    info!("{}", config.name);
    info!("  version {}", config.full_version());
    info!("  by Stake Technologies, 2018-2019");
    info!("Chain specification: {}", config.chain_spec.name());
    info!("Node name: {}", config.name);
    info!("Roles: {}", display_role(&config));

    // Create the service. This is the most heavy initialization step.
    let service = crate::service::new_light(config)
        .map_err(|e| format!("{:?}", e))?;

    Ok(browser_utils::start_client(service))
}
