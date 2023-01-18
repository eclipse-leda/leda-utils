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
use async_priority_channel::{bounded, Receiver, Sender};
use cursive::views::Dialog;
use cursive::{traits::*, Cursive};
use kantui::{containers_table_view as table, kanto_api, kantui_config, try_best};
use nix::unistd::Uid;

#[derive(Debug)]
enum KantoRequest {
    ListContainers,
    _CreateContainer(String, String), // Name, Registry
    StartContainer(String),           // ID
    StopContainer(String, i64),       // ID, timeout
    RemoveContainer(String),          // ID
    GetLogs(String),                  // ID
}

#[derive(Debug)]
enum KantoResponse {
    ListContainers(Vec<kanto_api::Container>),
    GetLogs(String),
}

#[derive(PartialOrd, Ord, PartialEq, Eq, Debug)]
enum RequestPriority {
    Low = 0,
    Normal = 10,
    _High = 50,
}

/// IO Thread
/// Parses requests from the UI thread sent to the request channel and sends the results
/// back to the response channel. This two-channel architecture allows us to set up non-blocking
/// communication between async and sync code.
#[cfg(unix)]
#[tokio::main]
async fn tokio_main(
    response_tx: Sender<KantoResponse, RequestPriority>,
    request_rx: &mut Receiver<KantoRequest, RequestPriority>,
    config: kantui_config::AppConfig,
) -> kanto_api::Result<()> {
    let mut c = kanto_api::get_connection(&config.socket_path).await?;
    loop {
        if let Ok((request, _)) = request_rx.recv().await {
            match request {
                KantoRequest::ListContainers => {
                    let r = kantui::kanto_api::list_containers(&mut c).await?;
                    try_best(
                        response_tx
                            .send(KantoResponse::ListContainers(r), RequestPriority::Low)
                            .await?,
                    );
                }
                KantoRequest::_CreateContainer(id, registry) => {
                    try_best(kanto_api::create_container(&mut c, &id, &registry).await);
                }
                KantoRequest::StartContainer(id) => {
                    try_best(kanto_api::start_container(&mut c, &id).await);
                }
                KantoRequest::StopContainer(id, timeout) => {
                    try_best(kanto_api::stop_container(&mut c, &id, timeout).await);
                }
                KantoRequest::RemoveContainer(id) => {
                    try_best(kanto_api::remove_container(&mut c, &id, true).await);
                }
                KantoRequest::GetLogs(id) => {
                    let logs = match kanto_api::get_container_logs(&id).await {
                        Ok(logs) => logs,
                        Err(_) => "Could not obtain logs".to_string(),
                    };
                    try_best(
                        response_tx
                            .send(KantoResponse::GetLogs(logs), RequestPriority::Normal)
                            .await,
                    );
                }
            }
        }
    }
}

/// Setup the user interface and start the UI thread
fn run_ui(
    tx_requests: Sender<KantoRequest, RequestPriority>,
    rx_responses: Receiver<KantoResponse, RequestPriority>,
    config: kantui_config::AppConfig,
) -> kanto_api::Result<()> {
    let mut siv = cursive::default();

    table::set_cursive_theme(&mut siv);

    let table = table::generate_table_view();

    let start_cb = enclose::enclose!((tx_requests) move |s: &mut Cursive| {
        if let Some(c) = table::get_current_container(s) {
            try_best(tx_requests.try_send(KantoRequest::StartContainer(c.id.clone()), RequestPriority::Normal));
        }
    });

    let stop_cb = enclose::enclose!((tx_requests) move |s: &mut Cursive| {
        if let Some(c) = table::get_current_container(s) {
            try_best(tx_requests.try_send(KantoRequest::StopContainer(c.id.clone(), config.stop_timeout as i64), RequestPriority::Normal));
        }
    });

    let remove_cb = enclose::enclose!((tx_requests) move |s: &mut Cursive| {
        if let Some(c) = table::get_current_container(s) {
            try_best(tx_requests.try_send(KantoRequest::RemoveContainer(c.id.clone()), RequestPriority::Normal));
        }
    });

    let get_logs_cb = enclose::enclose!((tx_requests) move |s: &mut Cursive| {
        if let Some(c) = table::get_current_container(s) {
            try_best(tx_requests.try_send(KantoRequest::GetLogs(c.id.clone()), RequestPriority::Normal));
        }
    });

    siv.add_fullscreen_layer(
        Dialog::around(table.with_name(table::TABLE_IDENTIFIER).full_screen())
            .title("Kanto Container Management")
            .button(config.keyconfig.start_btn_name, start_cb.clone())
            .button(config.keyconfig.stop_btn_name, stop_cb.clone())
            .button(config.keyconfig.remove_btn_name, remove_cb.clone())
            .button(config.keyconfig.logs_btn_name, get_logs_cb.clone())
      //      .button(config.keyconfig.redeploy_btn_name, get_logs_cb.clone())
            .button(config.keyconfig.quit_btn_name, |s| s.quit()),
    );

    // Add keyboard shortcuts
    siv.add_global_callback(config.keyconfig.start_kbd_key, start_cb.clone());
    siv.add_global_callback(config.keyconfig.stop_kbd_key, stop_cb.clone());
    siv.add_global_callback(config.keyconfig.remove_kbd_key, remove_cb.clone());
    siv.add_global_callback(config.keyconfig.logs_kbd_key, get_logs_cb.clone());
    //siv.add_global_callback(config.keyconfig.redeploy_kbd_key, get_logs_cb.clone());
    siv.add_global_callback(config.keyconfig.quit_kbd_key, |s| s.quit());

    siv.set_fps(30);

    siv.add_global_callback(cursive::event::Event::Refresh, move |s| {
        try_best(tx_requests.try_send(KantoRequest::ListContainers, RequestPriority::Low));
        if let Ok((resp, _)) = rx_responses.try_recv() {
            match resp {
                KantoResponse::ListContainers(list) => table::update_table_items(s, list),
                KantoResponse::GetLogs(logs) => table::show_logs_view(s, logs),
            }
        }
    });

    siv.try_run_with(table::buffered_termion_backend)?;

    Ok(())
}

fn main() -> kanto_api::Result<()> {
    let config = kantui_config::get_app_configuration()?;

    if !Uid::effective().is_root() {
        eprintln!("You must run this executable as root");
        std::process::exit(-1);
    }

    let (tx_responses, rx_responses) = bounded::<KantoResponse, RequestPriority>(5);
    let (tx_requests, mut rx_requests) = bounded::<KantoRequest, RequestPriority>(5);

    // Give each thread its own copy of the config.
    std::thread::spawn(enclose::enclose!((config) move || {
        tokio_main(tx_responses, &mut rx_requests, config).expect("Error in io thread");
    }));

    run_ui(tx_requests, rx_responses, config)?;

    Ok(())
}
