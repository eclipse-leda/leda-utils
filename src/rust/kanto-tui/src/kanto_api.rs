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
use anyhow::anyhow;
#[cfg(unix)]
use strip_ansi_escapes::strip;
use tokio::net::UnixStream;
use tonic::transport::{Endpoint, Uri};
use tower::service_fn;

use super::{cm_rpc, cm_types, ClientChannel, Result};

const CONT_TEMPLATE: &'static str = include_str!("container_json_template.in");

pub async fn get_connection(socket_path: &str) -> Result<ClientChannel> {
    let _path = socket_path.to_owned();
    let channel = Endpoint::try_from("http://[::]:50051")?
        .connect_with_connector(service_fn(move |_: Uri| UnixStream::connect(_path.clone())))
        .await?;
    Ok(cm_rpc::containers_client::ContainersClient::new(channel))
}

pub async fn list_containers(channel: &mut ClientChannel) -> Result<Vec<cm_types::Container>> {
    let _r = tonic::Request::new(cm_rpc::ListContainersRequest {});
    let containers = channel.list(_r).await?.into_inner();
    Ok(containers.containers)
}

pub async fn create_container(
    channel: &mut ClientChannel,
    name: &str,
    registry: &str,
) -> Result<cm_rpc::CreateContainerResponse> {
    let mut template: cm_types::Container = serde_json::from_str(CONT_TEMPLATE)?;
    template.name = String::from(name);
    template.image.as_mut().ok_or("Field name missing")?.name = String::from(registry);

    let _r = tonic::Request::new(cm_rpc::CreateContainerRequest {
        container: Some(template),
    });
    let _response = channel.create(_r).await?;
    Ok(_response.into_inner())
}

pub async fn get_container_by_id(
    channel: &mut ClientChannel,
    id: &str,
) -> Result<cm_types::Container> {
    let _r = tonic::Request::new(cm_rpc::GetContainerRequest {
        id: String::from(id),
    });

    let cont = channel
        .get(_r)
        .await?
        .into_inner()
        .container
        .ok_or(anyhow!("No container with {id} found"))?;
    Ok(cont)
}

pub async fn start_container(channel: &mut ClientChannel, id: &str) -> Result<()> {
    let _r = tonic::Request::new(cm_rpc::StartContainerRequest {
        id: String::from(id),
    });
    let _r = channel.start(_r).await?;
    Ok(())
}

pub async fn stop_container(channel: &mut ClientChannel, id: &str, timeout: i64) -> Result<()> {
    let stop_options = Some(cm_types::StopOptions {
        timeout,
        force: true,
        signal: String::from("SIGTERM"),
    });

    let _r = tonic::Request::new(cm_rpc::StopContainerRequest {
        id: String::from(id),
        stop_options,
    });
    let _r = channel.stop(_r).await?;
    Ok(())
}

pub async fn remove_container(channel: &mut ClientChannel, id: &str, force: bool) -> Result<()> {
    let _r = tonic::Request::new(cm_rpc::RemoveContainerRequest {
        id: String::from(id),
        force,
    });
    let _r = channel.remove(_r).await?;
    Ok(())
}

pub async fn redeploy_containers(redeploy_command: &str) -> Result<()> {
    let mut lex = shlex::Shlex::new(redeploy_command);
    let shell_words = lex.by_ref().collect::<Vec<_>>();

    if lex.had_error {
        return Err(Box::from(anyhow!("Failed parsing redeploy command")));
    }

    tokio::process::Command::new(&shell_words[0])
        .args(&shell_words[1..])
        .spawn()?
        .wait()
        .await?;
    Ok(())
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct KantoLogLine {
    stream: String,
    log: String,
    time: String,
}

// Strips the console control characters and updates the log string
fn strip_and_push(line: &KantoLogLine, log: &mut String) {
    let stripped: Vec<u8> = strip(line.log.clone()).unwrap();
    log.push_str(String::from_utf8_lossy(&stripped).as_ref());
}

pub async fn get_logs(channel: &mut ClientChannel, id: &str, tail: i32) -> Result<String> {
    let _r = tonic::Request::new(cm_rpc::GetLogsRequest {
        id: String::from(id),
        tail: tail as i32,
    });
    let mut _r = channel.logs(_r).await?.into_inner();
    let mut log = String::new();
    while let Some(msg) = _r.message().await? {
        let parsed_line = serde_json::from_str(&msg.log)?;
        strip_and_push(&parsed_line, &mut log);
    }
    Ok(log)
}
