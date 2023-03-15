// /********************************************************************************
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
// ********************************************************************************/
pub mod containers_table_view;
pub mod kanto_api;
pub mod kantui_config;
pub mod io;
pub mod ui;


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
    GetLogs(String),                  // ID
    Redeploy,
}

#[derive(Debug)]
pub enum KantoResponse {
    ListContainers(Vec<kanto_api::Container>),
    GetLogs(String),
}

#[derive(PartialOrd, Ord, PartialEq, Eq, Debug)]
pub enum RequestPriority {
    Low = 0,
    Normal = 10,
    _High = 50,
}