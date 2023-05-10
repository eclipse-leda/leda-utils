// ********************************************************************************
// * Copyright (c) 2023 Contributors to the Eclipse Foundation
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
use cursive::{traits::*, Cursive};
use super::{
    kantui_config::AppConfig, try_best, KantoRequest, KantoResponse, RequestPriority, Result,
};
use async_priority_channel::{Receiver, Sender};
use cursive::views::Dialog;

pub mod containers_table_view;
use containers_table_view as table;

/// Setup the user interface and start the UI thread
pub fn run(
    tx_requests: Sender<KantoRequest, RequestPriority>,
    rx_responses: Receiver<KantoResponse, RequestPriority>,
    config: AppConfig,
) -> Result<()> {
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
            try_best(tx_requests.try_send(KantoRequest::GetLogs(c.id.clone(), config.log_tail_lines as i32), RequestPriority::Normal));
        }
    });

    let describe_cb = enclose::enclose!((tx_requests) move |s: &mut Cursive| {
        if let Some(c) = table::get_current_container(s) {
            try_best(tx_requests.try_send(KantoRequest::GetFullContainerState(c.id.clone()), RequestPriority::Normal));
        }
    });

    let redeploy_cb = enclose::enclose!((tx_requests) move |_s: &mut Cursive| {
        try_best(tx_requests.try_send(KantoRequest::Redeploy, RequestPriority::Normal));
    });
    let help_cb =
        enclose::enclose!((config) move |s: &mut Cursive| table::help_screen(s, config.clone()));

    siv.add_fullscreen_layer(
        Dialog::around(table.with_name(table::TABLE_IDENTIFIER).full_screen())
            .title("Kanto Container Management")
            .button(config.keyconfig.start_btn_name, start_cb.clone())
            .button(config.keyconfig.stop_btn_name, stop_cb.clone())
            .button(config.keyconfig.remove_btn_name, remove_cb.clone())
            .button(config.keyconfig.logs_btn_name, get_logs_cb.clone())
            .button(config.keyconfig.redeploy_btn_name, redeploy_cb.clone())
            .button(config.keyconfig.describe_btn_name, describe_cb.clone())
            .button(config.keyconfig.help_btn_name, help_cb.clone())
            .button(config.keyconfig.quit_btn_name, |s| s.quit())
            .scrollable()
            .scroll_x(true)
            .scroll_y(true),
    );
    // Add keyboard shortcuts
    siv.add_global_callback(config.keyconfig.start_kbd_key, start_cb.clone());
    siv.add_global_callback(config.keyconfig.stop_kbd_key, stop_cb.clone());
    siv.add_global_callback(config.keyconfig.remove_kbd_key, remove_cb.clone());
    siv.add_global_callback(config.keyconfig.logs_kbd_key, get_logs_cb.clone());
    siv.add_global_callback(config.keyconfig.redeploy_kbd_key, redeploy_cb.clone());
    siv.add_global_callback(config.keyconfig.describe_kbd_key, describe_cb.clone());
    siv.add_global_callback(config.keyconfig.help_kbd_key, help_cb.clone());

    siv.add_global_callback(config.keyconfig.quit_kbd_key, |s| s.quit());

    siv.set_fps(config.container_list_refresh_fps.into());

    siv.add_global_callback(cursive::event::Event::Refresh, move |s| {
        try_best(tx_requests.try_send(KantoRequest::ListContainers, RequestPriority::Low));
        if let Ok((resp, _)) = rx_responses.try_recv() {
            match resp {
                KantoResponse::ListContainers(list) => table::update_table_items(s, list),
                KantoResponse::GetLogs(logs) => table::show_logs_view(s, logs),
                KantoResponse::GetFullContainerState(container) => {
                    table::describe_screen(s, container)
                }
            }
        }
    });

    siv.try_run_with(table::buffered_termion_backend)?;

    Ok(())
}
