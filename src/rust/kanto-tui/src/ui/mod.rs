use cursive::views::Dialog;
use cursive::{traits::*, Cursive};
use super::{
    Result,
    try_best, KantoRequest, KantoResponse, RequestPriority,
    kantui_config,
};

use async_priority_channel::{Receiver, Sender};

pub mod containers_table_view;
use containers_table_view as table;


/// Setup the user interface and start the UI thread
pub fn run(
    tx_requests: Sender<KantoRequest, RequestPriority>,
    rx_responses: Receiver<KantoResponse, RequestPriority>,
    config: kantui_config::AppConfig,
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
            try_best(tx_requests.try_send(KantoRequest::GetLogs(c.id.clone()), RequestPriority::Normal));
        }
    });

    let redeploy_cb = enclose::enclose!((tx_requests) move |_s: &mut Cursive| {
        try_best(tx_requests.try_send(KantoRequest::Redeploy, RequestPriority::Normal));
    });

    siv.add_fullscreen_layer(
        Dialog::around(table.with_name(table::TABLE_IDENTIFIER).full_screen())
            .title("Kanto Container Management")
            .button(config.keyconfig.start_btn_name, start_cb.clone())
            .button(config.keyconfig.stop_btn_name, stop_cb.clone())
            .button(config.keyconfig.remove_btn_name, remove_cb.clone())
            .button(config.keyconfig.logs_btn_name, get_logs_cb.clone())
            .button(config.keyconfig.redeploy_btn_name, redeploy_cb.clone())
            .button(config.keyconfig.quit_btn_name, |s| s.quit()),
    );

    // Add keyboard shortcuts
    siv.add_global_callback(config.keyconfig.start_kbd_key, start_cb.clone());
    siv.add_global_callback(config.keyconfig.stop_kbd_key, stop_cb.clone());
    siv.add_global_callback(config.keyconfig.remove_kbd_key, remove_cb.clone());
    siv.add_global_callback(config.keyconfig.logs_kbd_key, get_logs_cb.clone());
    siv.add_global_callback(config.keyconfig.redeploy_kbd_key, redeploy_cb.clone());
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