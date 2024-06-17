use std::{collections::HashSet, task::Poll};

use cargo::{
    core::{SourceId, Workspace},
    ops::RegistryOrIndex,
    sources::{registry::RegistrySource, source::Source},
    util::Config,
};
use cargo_credential::Secret;
use clap::{
    builder::{styling::AnsiColor, Styles},
    Parser,
};
use clap_cargo::Manifest;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

fn styles() -> Styles {
    Styles::plain()
        .header(AnsiColor::Yellow.on_default())
        .error(AnsiColor::Red.on_default().bold())
        .usage(AnsiColor::Yellow.on_default())
        .literal(AnsiColor::Green.on_default())
        .placeholder(AnsiColor::Green.on_default())
        .valid(AnsiColor::Green.on_default())
        .invalid(AnsiColor::Yellow.on_default())
}

#[derive(Debug, Parser)]
#[clap(name = "cargo", bin_name = "cargo", styles = styles())]
pub enum Command {
    KtraLogin(KtraLoginOpt),
}

/// Automated login for private ktra registries
#[derive(Debug, Clone, clap::Args)]
#[clap(author, about, long_about = None, version)]
pub struct KtraLoginOpt {
    #[clap(flatten)]
    manifest: Manifest,
    /// The user account to log in with
    username: String,
    /// The account password to log in with
    password: String,
    /// Check that the manifest is valid and that the remote registry exists, but don't generate a token
    #[clap(long)]
    dry_run: bool,
    /// The name of the registry to log in to
    #[clap(long)]
    registry: Option<String>,
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
    let registry = login
        .registry
        .or(config.default_registry()?)
        .ok_or(anyhow::anyhow!(
            "No default registry is set for this package"
        ))?;
    let source_id = SourceId::alt_registry(config, &registry)?;
    let lock = config.acquire_package_cache_lock()?;
    let mut registry_source = RegistrySource::remote(source_id, &HashSet::new(), config)?;
    registry_source.invalidate_cache();
    registry_source.block_until_ready()?;
    let api = loop {
        match registry_source.config()? {
            Poll::Pending => registry_source.block_until_ready()?,
            Poll::Ready(cfg) => break cfg,
        }
    }
    .and_then(|cfg| cfg.api)
    .ok_or(anyhow::anyhow!("invalid registry"))?;
    drop(lock);

    let request = Client::new();
    // In a dry run, don't include the password field to avoid token generation
    let remote_resp: KtraResponse = request
        .post(format!("{}/ktra/api/v1/login/{}", api, login.username))
        .header("content-type", "application/json")
        .body(if !login.dry_run {
            format!("{{\"password\": \"{}\"}}", login.password)
        } else {
            "{}".to_owned()
        })
        .send()?
        .json()?;

    if login.dry_run {
        Ok(())
    } else {
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
                Some(Secret::from(token.as_ref())),
                Some(&RegistryOrIndex::Registry(registry)),
                &[],
            )?),
        }
    }
}
