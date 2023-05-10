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
pub mod io;
pub mod kanto_api;
pub mod kantui_config;
pub mod ui;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
pub type ClientChannel = cm_rpc::containers_client::ContainersClient<tonic::transport::Channel>;

// This is a re-export to allow for the compilation and inclusion of deeply nested protobufs
mod containers {
    tonic::include_proto!("mod");
}
pub use containers::github::com::eclipse_kanto::container_management::containerm::api::services::containers as cm_rpc;
pub use containers::github::com::eclipse_kanto::container_management::containerm::api::types::containers as cm_types;

pub fn try_best<T>(err: T) {
    // Used to consume Err variants where they can be safely ignored.
    // Using it means that we try an operation to the best of our abilities
    // but failures can be (safely) ignored. E.g. we try to send a request down a
    // channel but if it's full we don't do anything
    std::mem::drop(err);
}

#[derive(Debug)]
pub enum KantoRequest {
    ListContainers,
    _CreateContainer(String, String), // Name, Registry
    StartContainer(String),           // ID
    StopContainer(String, i64),       // ID, timeout
    RemoveContainer(String),          // ID
    GetLogs(String, i32),             // ID, tail
    GetFullContainerState(String),    // ID
    Redeploy,
}

#[derive(Debug)]
pub enum KantoResponse {
    ListContainers(Vec<cm_types::Container>),
    GetLogs(String),
    GetFullContainerState(cm_types::Container),
}

#[derive(PartialOrd, Ord, PartialEq, Eq, Debug)]
pub enum RequestPriority {
    Low = 0,
    Normal = 10,
    _High = 50,
}
