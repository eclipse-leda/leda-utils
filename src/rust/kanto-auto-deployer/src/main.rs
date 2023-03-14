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
#[cfg(unix)]
use tokio::net::UnixStream;
use tonic::transport::{Endpoint, Uri};
use tower::service_fn;

pub mod containers {
    //This is a hack because tonic has an issue with deeply nested protobufs
    tonic::include_proto!("mod");
}
use containers::github::com::eclipse_kanto::container_management::containerm::api::services::containers as kanto;
use containers::github::com::eclipse_kanto::container_management::containerm::api::types::containers as kanto_cnt;
use glob::glob;
use std::env;
use std::fs;

fn print_usage() {
    println!("USAGE:");
    println!("  kanto-auto-deployer [PATH TO MANIFESTS FOLDER]")
}

async fn start(
    _client: &mut kanto::containers_client::ContainersClient<tonic::transport::Channel>,
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
    _client: &mut kanto::containers_client::ContainersClient<tonic::transport::Channel>,
    file_path: &String,
) -> Result<(), Box<dyn std::error::Error>> {
    let container_str = fs::read_to_string(file_path)?;
    let parsed_json = manifest_parser::try_parse_manifests(&container_str);
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or("AUTODEPLOYER_LOG", "info"),
    );

    let args: Vec<String> = env::args().collect();
    let mut file_path = String::new();
    let mut path = String::new();
    if args.len() == 2 {
        file_path.push_str(&args[1]);
        if file_path.eq("--help") || file_path.eq("-h") {
            print_usage();
            return Ok(());
        }
        path.push_str(&file_path.clone());
    } else {
        file_path.push_str(".");
        path.push_str(&file_path.clone());
    }
    file_path.push_str("/*.json");

    let socket_path = "/run/container-management/container-management.sock";
    //The uri is ignored and a UDS connection is established instead.
    let channel = Endpoint::try_from("http://[::]:50051")?
        .connect_with_connector(service_fn(move |_: Uri| UnixStream::connect(socket_path)))
        .await?;

    // Get the client
    let mut client = kanto::containers_client::ContainersClient::new(channel);
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
        match create(&mut client, &full_name).await {
            Ok(_) => {}
            Err(e) => log::error!("[CM error] Failed to create container: {}", e),
        };
        full_name.clear();
    }
    if !b_found {
        log::info!("No manifests are found in [{}]", path);
        print_usage();
    }
    Ok(())
}
