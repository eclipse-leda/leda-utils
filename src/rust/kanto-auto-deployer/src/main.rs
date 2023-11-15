// ********************************************************************************
// * Copyright (c) 2022 Contributors to the Eclipse Foundation
// *
// * See the NOTICE file(s) distributed with this work for additional
// * information regarding copyright ownership.
// *
// * This program and the accompanying materials are made available under the
// * terms of the Apache License 2.0 which is available at
// * https://www.apache.org/licenses/LICENSE-2.0
// *
// * SPDX-License-Identifier: Apache-2.0
// ********************************************************************************
use glob::glob;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use clap::Parser;
use futures::future;
use std::sync::atomic::AtomicBool;
use tokio::net::UnixStream;
use tokio_retry::{strategy, RetryIf};
use tonic::transport::{Endpoint, Uri};
use tower::service_fn;

#[cfg(feature = "mqtt")]
use clap::Args;
#[cfg(feature = "mqtt")]
use std::thread;

#[cfg(feature = "mqtt")]
pub mod mqtt_listener;
pub mod containers {
    //This is a hack because tonic has an issue with deeply nested protobufs
    tonic::include_proto!("mod");
}

#[cfg(feature = "filewatcher")]
pub mod fs_watcher;
#[cfg(feature = "filewatcher")]
use fs_watcher::is_filetype;

pub mod manifest_parser;

use containers::github::com::eclipse_kanto::container_management::containerm::api::services::containers as kanto;
use containers::github::com::eclipse_kanto::container_management::containerm::api::types::containers as kanto_cnt;

type CmClient = kanto::containers_client::ContainersClient<tonic::transport::Channel>;

#[derive(Parser, Debug)]
#[clap(version, about)]
pub struct CliArgs {
    /// Set the path to the directory containing the manifests
    #[clap(default_value = ".")]
    manifests_path: PathBuf,

    /// Set the path to the Kanto Container Management API socket
    #[clap(
        long,
        short,
        action,
        default_value = "/run/container-management/container-management.sock"
    )]
    socket_cm: PathBuf,

    /// Run as a daemon that continuously monitors the provided path for changes
    #[clap(long, short, action, default_value_t = false)]
    #[cfg(feature = "filewatcher")]
    daemon: bool,

    #[cfg(feature = "mqtt")]
    #[clap(flatten)]
    mqtt: MQTTconfig,
}

#[cfg(feature = "mqtt")]
#[derive(Debug, Args)]
pub struct MQTTconfig {
    /// Enable an MQTT client that listens for the desired state message and disables kanto-auto-deployer to avoid conflicts
    #[clap(short = 'm', long = "mqtt")]
    enabled: bool,

    /// Hostname/IP to the MQTT broker where the desired state message would be posted
    #[clap(long = "mqtt-broker-host", default_value = "localhost")]
    ip: String,

    /// Port for the MQTT broker
    #[clap(long = "mqtt-broker-port", default_value_t = 1883)]
    port: u16,

    /// Topic on which to subscribe for the desired state message feedback
    #[clap(
        long = "mqtt-topic",
        default_value = "containersupdate/desiredstatefeedback"
    )]
    topic: String,
}

static CM_RETRY_BASE_TIMEOUT_MS: u64 = 100;
// Conditional compilation would give warnings for unused variants
#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq)]
pub enum RetryTimes {
    Count(u32),
    Forever,
    Never,
}

struct RetryState {
    retry_times: RetryTimes,
}

impl RetryState {
    fn new(retry_times: RetryTimes) -> Self {
        RetryState { retry_times }
    }

    // Updates the count and returns true if the caller should stop retrying
    fn tick(&mut self) -> bool {
        match self.retry_times {
            RetryTimes::Forever => true,
            RetryTimes::Never => false,
            RetryTimes::Count(c) => {
                let retries_left = c.saturating_sub(1);
                self.retry_times = RetryTimes::Count(retries_left);
                retries_left > 0
            }
        }
    }
}

async fn get_unix_channel(socket_path: &str) -> Result<tonic::transport::Channel> {
    let socket_path = PathBuf::from(socket_path);
    let channel = Endpoint::try_from("http://[::]:50051")?
        .connect_with_connector(service_fn(move |_: Uri| {
            UnixStream::connect(socket_path.clone())
        }))
        .await?;
    Ok(channel)
}

async fn get_client(socket_path: &str, retries: RetryTimes) -> Result<CmClient> {
    let mut retry_state = RetryState::new(retries);
    let retry_strategy = strategy::FibonacciBackoff::from_millis(CM_RETRY_BASE_TIMEOUT_MS)
        .map(|d| {
            log::debug!("Retrying connection in {} ms", d.as_millis());
            d
        })
        .take_while(|_| retry_state.tick());

    let channel = RetryIf::spawn(
        retry_strategy,
        || async { get_unix_channel(socket_path).await },
        |e: &anyhow::Error| {
            log::error!(
                "An error occurred when connecting to socket: {:?}",
                e.root_cause()
            );
            true
        },
    )
    .await?;

    let client = kanto::containers_client::ContainersClient::new(channel);
    Ok(client)
}

async fn start(_client: &mut CmClient, name: &str, _id: &str) -> Result<()> {
    log::info!("Starting [{}]", name);
    let id = String::from(_id);
    let request = tonic::Request::new(kanto::StartContainerRequest { id });
    let _response = _client.start(request).await?;
    log::info!("Started [{}]", name);
    Ok(())
}

pub async fn stop(_client: &mut CmClient, id: &str, timeout: i64) -> Result<()> {
    let stop_options = Some(kanto_cnt::StopOptions {
        timeout,
        force: true,
        signal: String::from("SIGTERM"),
    });

    let _r = tonic::Request::new(kanto::StopContainerRequest {
        id: String::from(id),
        stop_options,
    });
    let _r = _client.stop(_r).await?;
    Ok(())
}

pub async fn remove(_client: &mut CmClient, id: &str) -> Result<()> {
    let _r = tonic::Request::new(kanto::RemoveContainerRequest {
        id: String::from(id),
        force: true,
    });
    let _r = _client.remove(_r).await?;
    Ok(())
}

fn container_running(c: &kanto_cnt::Container) -> bool {
    if let Some(state) = &c.state {
        return state.running;
    }
    false
}

async fn handle_existing(
    _client: &mut CmClient,
    new_cont: kanto_cnt::Container,
    existing_cont: &kanto_cnt::Container,
    recreate: bool,
) -> Result<()> {
    log::info!("Already exists [{}]", &new_cont.name);
    if !recreate {
        // If we do not wish to recreate the container only start it if needed
        // and return early
        log::debug!("Skipping {}", &new_cont.name);
        if !container_running(existing_cont) {
            start(_client, &new_cont.name, &existing_cont.id).await?;
        }
        return Ok(());
    }
    if container_running(existing_cont) {
        log::debug!("Stopping [{}]", &new_cont.name);
        stop(_client, &existing_cont.id, 1).await?;
    }
    log::info!("Removing [{}]", &new_cont.name);
    remove(_client, &existing_cont.id).await?;
    deploy_new(_client, new_cont).await?;
    Ok(())
}

async fn deploy_new(_client: &mut CmClient, new_cont: kanto_cnt::Container) -> Result<()> {
    let new_cont_name = new_cont.name.clone();
    log::info!("Creating [{}]", &new_cont_name);
    let request = tonic::Request::new(kanto::CreateContainerRequest {
        container: Some(new_cont),
    });
    let _response = _client.create(request).await?;
    log::info!("Created [{}]", &new_cont_name);
    let id = match _response.into_inner().container {
        Some(c) => c.id,
        None => String::new(),
    };
    start(_client, &new_cont_name, &id).await?;
    Ok(())
}

async fn deploy(socket: &str, retries: RetryTimes, file_path: &str, recreate: bool) -> Result<()> {
    let container_str = tokio::fs::read_to_string(file_path).await?;
    let mut _client = get_client(socket, retries).await?;
    let parsed_json = manifest_parser::try_parse_manifest(&container_str);
    if let Ok(new_container) = parsed_json {
        let _r = tonic::Request::new(kanto::ListContainersRequest {});
        let containers_list = _client.list(_r).await?.into_inner().containers;
        let existing_instance = containers_list
            .iter()
            .find(|c| c.name == new_container.name);
        if let Some(existing_cont) = existing_instance {
            handle_existing(&mut _client, new_container, existing_cont, recreate).await
        } else {
            deploy_new(&mut _client, new_container).await
        }
    } else {
        Err(anyhow::anyhow!("Wrong json in [{}]", file_path))
    }
}

async fn deploy_directory(directory_path: &str, socket: &str, retries: RetryTimes) -> Result<()> {
    let mut file_path = String::from(directory_path);
    log::info!("Reading manifests from [{}]", file_path);
    file_path.push_str("/*.json");

    let found_paths: Vec<String> = glob(&file_path)?
        .filter_map(Result::ok)
        .filter_map(|path| Some(path.to_str()?.to_owned()))
        .collect();
    if found_paths.is_empty() {
        return Err(anyhow::anyhow!("No manifests found in {directory_path}"));
    }

    let deployments = future::join_all(
        found_paths
            .iter()
            .map(|p| deploy(socket, retries, p, false)),
    )
    .await;

    let (successful, failed): (Vec<_>, Vec<_>) = deployments.into_iter().partition(Result::is_ok);

    log::debug!(
        "Successfully deployed {}, Failed: {}, Out of {}",
        successful.len(),
        failed.len(),
        found_paths.len()
    );

    failed
        .iter()
        .for_each(|e| log::error!("[CM error] {:?}", e));

    if !failed.is_empty() {
        return Err(anyhow::anyhow!(
            "One or more deployments failed. Check the logs above for more information."
        ));
    }

    Ok(())
}

#[cfg(feature = "filewatcher")]
async fn redeploy_on_change(e: fs_watcher::Event, socket: &str) {
    // In daemon mode we wait until a connection is available to proceed
    // Unwrapping in this case is safe.
    for path in &e.paths {
        if !is_filetype(path, "json") {
            continue;
        }
        if e.kind.is_create() || e.kind.is_modify() {
            let json_path = String::from(path.to_string_lossy());
            if let Err(e) = deploy(socket, RetryTimes::Forever, &json_path, true).await {
                log::error!("[CM error] {:?}", e.root_cause());
            };
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
    );

    let cli = Arc::new(CliArgs::parse());
    log::debug!("{:#?}", cli);

    let socket_path = String::from(cli.socket_cm.to_string_lossy());
    let canonical_manifests_path = match std::fs::canonicalize(&cli.manifests_path) {
        Ok(p) => p,
        Err(e) => {
            log::error!(
                "Could not expand path {:#?}, err: {}",
                &cli.manifests_path,
                e
            );
            std::process::exit(-1);
        }
    };
    let manifests_path = String::from(canonical_manifests_path.to_string_lossy());

    log::info!("Running initial deployment of {:#?}", manifests_path);

    // Do not retry by default (CLI tool)
    let mut retry_times = RetryTimes::Never;

    #[cfg(feature = "filewatcher")]
    if cli.daemon {
        // If compiled with filewatcher and running as daemon, retry forever
        retry_times = RetryTimes::Forever
    }

    // One-shot deployment of all manifests in directory
    if let Err(e) = deploy_directory(&manifests_path, &socket_path, retry_times).await {
        log::error!("Failed to deploy directory: {e}")
    }

    #[cfg(feature = "filewatcher")]
    if cli.daemon {
        static THREAD_TERMINATE_FLAG: AtomicBool = AtomicBool::new(false);
        #[cfg(feature = "mqtt")]
        if cli.mqtt.enabled {
            thread::spawn({
                let cli = cli.clone();
                || mqtt_listener::mqtt_main(cli, &THREAD_TERMINATE_FLAG)
            });
        }
        log::info!(
            "Running in daemon mode. Continuously monitoring {:#?}",
            manifests_path
        );
        fs_watcher::async_watch(&THREAD_TERMINATE_FLAG, &manifests_path, |e| async {
            redeploy_on_change(e, &socket_path).await
        })
        .await?
    }

    Ok(())
}
