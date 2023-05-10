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
use crate::kantui_config::{AppConfig, ALT_REPR, CTRL_REPR};
use crate::{cm_types, try_best, Result};
use cursive::align::HAlign;
use cursive::view::Scrollable;
use cursive::views::{Dialog, OnEventView, TextView};
use cursive::With;
use cursive_buffered_backend;
use cursive_table_view::{TableView, TableViewItem};
use std::cmp::Ordering;
pub type CTView = TableView<ContainersTable, ContainerColumn>;

pub static TABLE_IDENTIFIER: &'static str = "table";

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub enum ContainerColumn {
    ID,
    Name,
    Image,
    State,
}

#[derive(Clone, Eq, Hash, PartialEq, Debug)]
pub struct ContainersTable {
    pub id: String,
    pub name: String,
    pub image: String,
    pub state: String,
}

impl TableViewItem<ContainerColumn> for ContainersTable {
    fn to_column(&self, column: ContainerColumn) -> String {
        match column {
            ContainerColumn::ID => self.id.to_string(),
            ContainerColumn::Name => self.name.to_string(),
            ContainerColumn::Image => self.image.to_string(),
            ContainerColumn::State => self.state.to_string(),
        }
    }

    fn cmp(&self, other: &Self, column: ContainerColumn) -> Ordering
    where
        Self: Sized,
    {
        match column {
            ContainerColumn::ID => self.id.cmp(&other.id),
            ContainerColumn::Name => self.name.cmp(&other.name),
            ContainerColumn::Image => self.image.cmp(&other.image),
            ContainerColumn::State => self.state.cmp(&other.state),
        }
    }
}

fn state_to_string(state: &Option<cm_types::State>) -> String {
    if let Some(state) = state {
        return state.status.clone();
    }

    String::from("Unknown?")
}
pub fn items_to_columns(req_items: Vec<cm_types::Container>) -> Vec<ContainersTable> {
    let mut out: Vec<ContainersTable> = vec![];

    for c in req_items {
        out.push(ContainersTable {
            id: c.id,
            name: c.name,
            image: c.image.map_or("N/A".to_string(), |image| image.name),
            state: state_to_string(&c.state),
        });
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

pub fn generate_table_view() -> CTView {
    CTView::new()
        .column(ContainerColumn::ID, "ID", |c| {
            c.align(HAlign::Center).width_percent(25)
        })
        .column(ContainerColumn::Name, "Name", |c| {
            c.align(HAlign::Center).width_percent(25)
        })
        .column(ContainerColumn::Image, "Image", |c| {
            c.align(HAlign::Center).width_percent(25)
        })
        .column(ContainerColumn::State, "State", |c| {
            c.align(HAlign::Center).width_percent(25)
        })
}

pub fn update_table_items(siv: &mut cursive::Cursive, list: Vec<cm_types::Container>) {
    let mut t = siv
        .find_name::<CTView>(TABLE_IDENTIFIER)
        .expect("Wrong table identifier?");
    let last_item = t.item();
    // Cache the position of the table selector
    t.set_items(items_to_columns(list));
    if let Some(idx) = last_item {
        // If such a position existed, set it where it was
        t.set_selected_item(idx);
    }
}

pub fn get_current_container(s: &mut cursive::Cursive) -> Option<ContainersTable> {
    let t = s
        .find_name::<CTView>(TABLE_IDENTIFIER)
        .expect("Wrong table identifier?");

    if let Some(container_idx) = t.item() {
        if let Some(container) = t.borrow_item(container_idx) {
            return Some(container.clone()); // small enough struct to be worth it
        }
    }

    None
}

pub fn set_cursive_theme(siv: &mut cursive::CursiveRunnable) {
    siv.set_theme(cursive::theme::Theme {
        shadow: true,
        borders: cursive::theme::BorderStyle::Simple,
        palette: cursive::theme::Palette::default().with(|palette| {
            use cursive::theme::BaseColor::*;
            use cursive::theme::Color::TerminalDefault;
            use cursive::theme::PaletteColor::*;

            palette[Background] = TerminalDefault;
            palette[View] = TerminalDefault;
            palette[Primary] = White.dark();
            palette[TitlePrimary] = Blue.light();
            palette[Secondary] = Blue.light();
            palette[Highlight] = Blue.dark();
        }),
    });
}

pub fn show_logs_view(siv: &mut cursive::Cursive, logs: String) {
    use cursive::event::Key::Esc;

    let mut logs_view = Dialog::around(TextView::new(logs))
        .title("Container Logs")
        .button("Ok (Esc)", |s| try_best(s.pop_layer()))
        .scrollable();

    logs_view.set_scroll_strategy(cursive::view::ScrollStrategy::StickToBottom);
    let logs_events_handler =
        OnEventView::new(logs_view).on_event(Esc, |s| try_best(s.pop_layer()));

    siv.add_layer(logs_events_handler);
}

fn host_config_description(host_config: &cm_types::HostConfig) -> String {
    format!(
        r"Network mode: {}
    Port Mappings: {:#?}
    Privileged: {},
    Devices: {:#?}",
        host_config.network_mode,
        host_config.port_mappings,
        host_config.privileged,
        host_config.devices
    )
}

pub fn describe_screen(siv: &mut cursive::Cursive, c: cm_types::Container) {
    use cursive::event::Key::Esc;
    let cnt_description = format!(
        r"
    General
    ========================
    ID: {}
    Name: {}
    Container hostname: {}
    Image: {}
    State: {}

    Host Config
    ========================
    {}

    Other
    =======================
    Mounts: {:#?}
    ",
        c.id,
        c.name,
        c.host_name,
        c.image.map_or("N/A".to_string(), |image| image.name),
        c.state.map_or("N/A".to_string(), |state| format!(
            "{} (Exit code: {})",
            state.status, state.exit_code
        )),
        c.host_config
            .map_or("N/A".to_string(), |config| host_config_description(&config)),
        c.mounts
    );

    let describe_view = Dialog::around(TextView::new(cnt_description))
        .title("Container Description")
        .button("Ok (Esc)", |s| try_best(s.pop_layer()))
        .scrollable();

    let describe_events_handler =
        OnEventView::new(describe_view).on_event(Esc, |s| try_best(s.pop_layer()));

    siv.add_layer(describe_events_handler);
}

pub fn help_screen(siv: &mut cursive::Cursive, config: AppConfig) {
    use cursive::event::Key::Esc;
    let help_string = format!(
        r"
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
        config.keyconfig.start_btn_name,
        config.keyconfig.start_kbd_key,
        config.keyconfig.stop_btn_name,
        config.keyconfig.stop_kbd_key,
        config.keyconfig.remove_btn_name,
        config.keyconfig.remove_kbd_key,
        config.keyconfig.logs_btn_name,
        config.keyconfig.logs_kbd_key,
        config.keyconfig.redeploy_btn_name,
        config.keyconfig.redeploy_kbd_key,
        config.keyconfig.help_btn_name,
        config.keyconfig.help_kbd_key,
        config.keyconfig.quit_btn_name,
        config.keyconfig.quit_kbd_key
    );

    let help_view = Dialog::around(TextView::new(help_string))
        .title("Help")
        .button("Ok (Esc)", |s| try_best(s.pop_layer()))
        .scrollable()
        .scroll_y(true)
        .scroll_x(true);

    let help_events_handler =
        OnEventView::new(help_view).on_event(Esc, |s| try_best(s.pop_layer()));

    siv.add_layer(help_events_handler);
}

pub fn buffered_termion_backend() -> Result<Box<dyn cursive::backend::Backend>> {
    let backend = cursive::backends::termion::Backend::init()?;
    let buffered_backend = cursive_buffered_backend::BufferedBackend::new(backend);
    Ok(Box::new(buffered_backend))
}
