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
use clap::Parser;
use cursive::views::Dialog;
use cursive::{traits::*, Cursive};
use kantui::{containers_table_view as table, kanto_api, try_best};
use nix::unistd::Uid;

#[derive(Parser, Debug)]
#[command(version, about)]
struct CliArgs {
    /// Set the path to the kanto-cm UNIX socket
    #[arg(short, long, default_value_t=String::from("/run/container-management/container-management.sock"))]
    socket: String,

    /// Time before sending a SIGKILL after a SIGTERM to a container (seconds)
    #[arg(short, long, default_value_t = 5)]
    timeout: u8,
}

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
    socket_path: &str,
) -> kanto_api::Result<()> {
    let mut c = kanto_api::get_connection(socket_path).await?;
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
    timeout: i64,
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
            try_best(tx_requests.try_send(KantoRequest::StopContainer(c.id.clone(), timeout), RequestPriority::Normal));
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
        Dialog::around(
            table
                .with_name(table::TABLE_IDENTIFIER)
                .min_size((400, 400)),
        )
        .title("Kanto Container Management")
        .button("[S]tart", start_cb.clone())
        .button("Sto[P]", stop_cb.clone())
        .button("[R]emove", remove_cb.clone())
        .button("[L]ogs", get_logs_cb.clone())
        .button("[Q]uit", |s| s.quit()),
    );

    // Add keyboard shortcuts
    siv.add_global_callback('s', start_cb.clone());
    siv.add_global_callback('p', stop_cb.clone());
    siv.add_global_callback('r', remove_cb.clone());
    siv.add_global_callback('l', get_logs_cb.clone());
    siv.add_global_callback('q', |s| s.quit());

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

    siv.run();

    Ok(())
}

fn main() -> kanto_api::Result<()> {
    let args = CliArgs::parse();

    if !Uid::effective().is_root() {
        eprintln!("You must run this executable as root");
        std::process::exit(-1);
    }

    let (tx_responses, rx_responses) = bounded::<KantoResponse, RequestPriority>(5);
    let (tx_requests, mut rx_requests) = bounded::<KantoRequest, RequestPriority>(5);

    std::thread::spawn(move || {
        tokio_main(tx_responses, &mut rx_requests, &args.socket).expect("Error in io thread");
    });

    run_ui(tx_requests, rx_responses, args.timeout as i64)?;

    Ok(())
}
