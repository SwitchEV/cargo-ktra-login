use std::collections::HashSet;

use cargo::{
    core::{source::SourceId, Source, Workspace},
    sources::registry::RegistrySource,
    util::{network::PollExt, Config},
};
use clap::Parser;
use clap_cargo::Manifest;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Parser)]
#[command(name = "cargo")]
#[command(bin_name = "cargo")]
pub enum Command {
    #[command(name = "ktra-login")]
    #[command(about, author, version)]
    KtraLogin(KtraLoginOpt),
}

/// Automated login for private ktra registries
#[derive(Debug, Clone, clap::Args)]
pub struct KtraLoginOpt {
    #[command(flatten)]
    manifest: Manifest,
    /// The user account to log in with
    username: String,
    /// The account password to log in with
    password: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum KtraResponse {
    Errors(Vec<KtraError>),
    Token(String),
}

#[derive(Debug, Deserialize, Serialize)]
struct KtraError {
    detail: String,
}

fn main() -> Result<(), anyhow::Error> {
    let args = Command::parse();
    let Command::KtraLogin(login) = args;
    let mut manifest = std::fs::canonicalize(
        login
            .manifest
            .manifest_path
            .unwrap_or(std::env::current_dir()?),
    )?;
    if manifest.is_dir() {
        manifest.push("Cargo.toml");
    }
    let config = Config::default()?;
    let ws = Workspace::new(&manifest, &config)?;
    let config = ws.config();
    let registry = config
        .default_registry()?
        .ok_or(anyhow::anyhow!("No default registry is set"))?;
    let source_id = SourceId::alt_registry(config, &registry)?;
    let lock = config.acquire_package_cache_lock()?;
    let mut registry_source = RegistrySource::remote(source_id, &HashSet::new(), config)?;
    let reg_config = registry_source.config();
    registry_source.block_until_ready()?;
    let api = reg_config
        .expect("Failed to get the registry source")?
        .unwrap()
        .api
        .unwrap();
    drop(lock);

    let request = Client::new();
    let remote_resp: KtraResponse = request
        .post(format!("{}/ktra/api/v1/login/{}", api, login.username))
        .header("content-type", "application/json")
        .body(format!("{{\"password\": \"{}\"}}", login.password))
        .send()?
        .json()?;

    match remote_resp {
        KtraResponse::Errors(e) => {
            let error = e
                .into_iter()
                .map(|e| e.detail)
                .collect::<Vec<String>>()
                .join(", ");
            Err(anyhow::anyhow!(error))?
        }
        KtraResponse::Token(token) => Ok(cargo::ops::registry_login(
            config,
            Some(token),
            Some(registry),
        )?),
    }
}
