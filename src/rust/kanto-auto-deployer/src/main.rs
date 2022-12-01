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
use std::fs;
use std::env;
use glob::glob;

async fn start(_client: &mut kanto::containers_client::ContainersClient<tonic::transport::Channel>, name: &String, _id: &String) -> Result<(), Box<dyn std::error::Error>> {

    println!("Starting [{}]", name);
    let id = String::from(_id.clone());
	let request = tonic::Request::new(kanto::StartContainerRequest{id});
	let _response  = _client.start(request).await?;
    println!("Started [{}]", name);
    Ok(())	
}

async fn create(_client: &mut kanto::containers_client::ContainersClient<tonic::transport::Channel>, file_path: &String) -> Result<(), Box<dyn std::error::Error>> {

    let container_str = fs::read_to_string(file_path)
        .expect("Should have been able to read the file");
	let container: kanto_cnt::Container = serde_json::from_str(&container_str)?;
	let name = String::from(container.name.clone());
	println!("Creating [{}]", name);
    let request = tonic::Request::new(kanto::CreateContainerRequest{container: Some(container)});
	let _response = _client.create(request).await?;
    println!("Created [{}]", name);
    let _none = String::new();
    let id = match _response.into_inner().container {
        Some(c) => c.id,
        None => _none
    };
    start(_client, &name, &id).await?;
    Ok(())	
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let mut file_path = String::new();
    let mut path = String::new();
    if args.len() == 2 {
        file_path.push_str(&args[1]);
        file_path.push_str("/");
        path.push_str(&file_path.clone())
    } else {
        file_path.push_str("./");
        path.push_str(&file_path.clone())
    }
    file_path.push_str("*.json");

    let socket_path = "/run/container-management/container-management.sock";
    //The uri is ignored and a UDS connection is established instead.
    let channel = Endpoint::try_from("http://[::]:50051")?
        .connect_with_connector(service_fn(move |_: Uri| UnixStream::connect(socket_path)))
        .await?;

    // Get the client
    let mut client = kanto::containers_client::ContainersClient::new(channel);

    let mut full_name = String::new();
    for entry in glob(&file_path)? {
        let name= entry?.display().to_string();
        full_name.push_str(&name);
        create(&mut client, &full_name).await?;
        full_name.clear()
    }

    Ok(())
}
