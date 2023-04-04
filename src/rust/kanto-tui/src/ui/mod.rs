use cursive::{traits::*, Cursive};
use super::{
    Result,
    try_best, KantoRequest, KantoResponse, RequestPriority,
    kantui_config::{AppConfig, CTRL_REPR, ALT_REPR},
};
use async_priority_channel::{Receiver, Sender};
use cursive::views::{Dialog, OnEventView, TextView};

pub mod containers_table_view;
use containers_table_view as table;

fn help_screen(siv: &mut Cursive, config: AppConfig) {
    use cursive::event::Key::Esc;
    let help_string = format!(r"
    You can use either the arrow keys/Tab/Enter (keyboard) 
    or the mouse (if your terminal supports mouse events) 
    to select a container from the list.

    UI Button/Keyboard Shortcut => Function
    ==================================================
    {}/{} => To Start the currently selected container
    {}/{} => To Stop the currently selected container
    {}/{} => To Remove the currently selected container
    {}/{} => To Get Logs for the currently selected container
    {}/{} => To Redeploy all container manifests
    {}/{} => To Display this help screen
    {}/{} => To Exit Kantui
    ==================================================

    Legend:
    ============================
    {CTRL_REPR}<key> = Ctrl+<key>
    {ALT_REPR}<key> = Alt+<key>
    ", 
    config.keyconfig.start_btn_name, config.keyconfig.start_kbd_key,
    config.keyconfig.stop_btn_name, config.keyconfig.stop_kbd_key,
    config.keyconfig.remove_btn_name, config.keyconfig.remove_kbd_key,
    config.keyconfig.logs_btn_name, config.keyconfig.logs_kbd_key,
    config.keyconfig.redeploy_btn_name, config.keyconfig.redeploy_kbd_key,
    config.keyconfig.help_btn_name, config.keyconfig.help_kbd_key,
    config.keyconfig.quit_btn_name, config.keyconfig.quit_kbd_key
    );

    let help_view = Dialog::around(TextView::new(help_string))
        .title("Help")
        .button("Ok (Esc)", |s| try_best(s.pop_layer()))
        .scrollable();

    let help_events_handler = OnEventView::new(help_view)
    .on_event(Esc, |s| try_best(s.pop_layer()));

    siv.add_layer(help_events_handler);
}

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
            try_best(tx_requests.try_send(KantoRequest::GetLogs(c.id.clone()), RequestPriority::Normal));
        }
    });

    let redeploy_cb = enclose::enclose!((tx_requests) move |_s: &mut Cursive| {
        try_best(tx_requests.try_send(KantoRequest::Redeploy, RequestPriority::Normal));
    });
    let help_cb = enclose::enclose!((config) move |s: &mut Cursive| help_screen(s, config.clone()));
    
    siv.add_fullscreen_layer(
        Dialog::around(table.with_name(table::TABLE_IDENTIFIER).full_screen())
        .title("Kanto Container Management")
        .button(config.keyconfig.start_btn_name, start_cb.clone())
        .button(config.keyconfig.stop_btn_name, stop_cb.clone())
        .button(config.keyconfig.remove_btn_name, remove_cb.clone())
        .button(config.keyconfig.logs_btn_name, get_logs_cb.clone())
        .button(config.keyconfig.redeploy_btn_name, redeploy_cb.clone())
        .button(config.keyconfig.help_btn_name, help_cb.clone())
        .button(config.keyconfig.quit_btn_name, |s| s.quit()),
    );
    // Add keyboard shortcuts
    siv.add_global_callback(config.keyconfig.start_kbd_key, start_cb.clone());
    siv.add_global_callback(config.keyconfig.stop_kbd_key, stop_cb.clone());
    siv.add_global_callback(config.keyconfig.remove_kbd_key, remove_cb.clone());
    siv.add_global_callback(config.keyconfig.logs_kbd_key, get_logs_cb.clone());
    siv.add_global_callback(config.keyconfig.redeploy_kbd_key, redeploy_cb.clone());
    siv.add_global_callback(config.keyconfig.help_kbd_key, help_cb.clone());
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