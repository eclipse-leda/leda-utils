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
pub mod manifest_parser;
use fs_watcher::is_filetype;
#[cfg(unix)]
use tokio::net::UnixStream;
use tonic::transport::{Endpoint, Uri};
use tower::service_fn;

pub mod containers {
    //This is a hack because tonic has an issue with deeply nested protobufs
    tonic::include_proto!("mod");
}
pub mod fs_watcher;

use clap::Parser;
use glob::glob;
use std::ffi::OsStr;
use std::fs;
use std::path::PathBuf;
type CmClient = kanto::containers_client::ContainersClient<tonic::transport::Channel>;
use containers::github::com::eclipse_kanto::container_management::containerm::api::services::containers as kanto;
use containers::github::com::eclipse_kanto::container_management::containerm::api::types::containers as kanto_cnt;

#[derive(Parser, Debug)]
#[clap(version, about)]
struct CliArgs {
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
    daemonize: bool,
}

async fn get_client(socket_path: &OsStr) -> Result<CmClient, Box<dyn std::error::Error>> {
    let socket_path = PathBuf::from(socket_path); // TODO: remove clone!
                                                  // TODO: remove clone!
    let channel = Endpoint::try_from("http://[::]:50051")?
        .connect_with_connector(service_fn(move |_: Uri| {
            UnixStream::connect(socket_path.clone())
        }))
        .await?;
    let client = kanto::containers_client::ContainersClient::new(channel);
    Ok(client)
}

async fn start(
    _client: &mut CmClient,
    name: &String,
    _id: &String,
) -> Result<(), Box<dyn std::error::Error>> {
    log::info!("Starting [{}]", name);
    let id = String::from(_id.clone());
    let request = tonic::Request::new(kanto::StartContainerRequest { id });
    let _response = _client.start(request).await?;
    log::info!("Started [{}]", name);
    Ok(())
}

async fn create(
    _client: &mut CmClient,
    file_path: &String,
) -> Result<(), Box<dyn std::error::Error>> {
    let container_str = fs::read_to_string(file_path)?;
    let parsed_json = manifest_parser::try_parse_manifest(&container_str);
    if let Ok(container) = parsed_json {
        let container: kanto_cnt::Container = container;
        let name = String::from(container.name.clone());
        let _r = tonic::Request::new(kanto::ListContainersRequest {});
        let containers = _client.list(_r).await?.into_inner();
        for cont in &containers.containers {
            if cont.name == name {
                log::info!("Already exists [{}]", name);
                return Ok(());
            }
        }
        log::info!("Creating [{}]", name);
        let request = tonic::Request::new(kanto::CreateContainerRequest {
            container: Some(container),
        });
        let _response = _client.create(request).await?;
        log::info!("Created [{}]", name);
        let _none = String::new();
        let id = match _response.into_inner().container {
            Some(c) => c.id,
            None => _none,
        };
        start(_client, &name, &id).await?;
    } else {
        log::error!("Wrong json in [{}]", file_path);
    }
    Ok(())
}

async fn deploy_directory(
    directory_path: &str,
    client: &mut CmClient,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut file_path = String::from(directory_path);
    let mut path = String::new();

    path.push_str(&file_path.clone());
    file_path.push_str("/*.json");

    let mut b_found = false;

    log::info!("Reading manifests from [{}]", path);

    let mut full_name = String::new();
    for entry in glob(&file_path).expect("Failed to parse glob pattern") {
        let name = entry
            .expect("Path to entry is unreadable")
            .display()
            .to_string();
        full_name.push_str(&name);
        b_found = true;
        match create(client, &full_name).await {
            Ok(_) => {}
            Err(e) => log::error!("[CM error] Failed to create container: {}", e),
        };
        full_name.clear();
    }
    if !b_found {
        log::error!("No manifests are found in [{}]", path);
    }
    Ok(())
}

async fn redeploy_on_change(e: fs_watcher::Event, manifests_path: String, mut client: CmClient) -> Result<(), Box<dyn std::error::Error>> {
    for path in &e.paths {
        if !is_filetype(path, "json") {
            continue;
        }
        if e.kind.is_create()  {
            todo!("Add logic for new manifest added");
        } 
        if e.kind.is_modify() {
            todo!("Add logic for old manifest modified");
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
    );

    let cli = CliArgs::parse();
    log::debug!("{:#?}", cli);
    
    let mut client = get_client(cli.socket_cm.as_os_str()).await?;
    let manifests_path = String::from(cli.manifests_path.to_string_lossy());

    log::info!("Running initial deployment of {:#?}", cli.manifests_path);
    deploy_directory(&manifests_path, &mut client).await?;

    if cli.daemonize {
        log::info!("Running in daemon mode. Continuously monitoring {:#?}", cli.manifests_path);
        fs_watcher::async_watch(&manifests_path, enclose::enclose!((manifests_path, client)|e| async move {
            todo!("Cleanup")
            // redeploy_on_change(e, manifests_path, client).await.unwrap()
        })).await?
    } 
    
    Ok(())
}
