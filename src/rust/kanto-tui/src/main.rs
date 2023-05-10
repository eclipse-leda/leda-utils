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
use async_priority_channel::bounded;
use kantui::{kantui_config, KantoRequest, KantoResponse, RequestPriority, Result};
use nix::unistd::Uid;

#[cfg(unix)]
fn main() -> Result<()> {
    let config = kantui_config::get_app_configuration()?;

    if !Uid::effective().is_root() {
        eprintln!("You must run this executable as root");
        std::process::exit(-1);
    }

    let (tx_responses, rx_responses) = bounded::<KantoResponse, RequestPriority>(5);
    let (tx_requests, mut rx_requests) = bounded::<KantoRequest, RequestPriority>(5);

    // Give each thread its own copy of the config.
    std::thread::spawn(enclose::enclose!((config) move || {
        kantui::io::async_io_thread(tx_responses, &mut rx_requests, config).expect("Error in io thread");
    }));

    kantui::ui::run(tx_requests, rx_responses, config)?;

    Ok(())
}
