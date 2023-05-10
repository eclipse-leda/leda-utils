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
use super::Result;
use clap::Parser;
use config::Config;
use cursive::event;
use serde::de;
use serde::Deserialize;
use serde::Serialize;
use std::fmt::Display;
use std::path::PathBuf;

pub const CTRL_REPR: char = '^';
pub const ALT_REPR: char = '@';

#[derive(Parser, Debug)]
#[clap(
    version,
    about,
    after_help = "Note: All config values can be overridden through env variables prefixed with KANTUI_,
e.g. KANTUI_STOP_TIMEOUT=5 overrides the timeout before SIGKILL is sent to be 5 seconds. \n\n"
)]
struct CliArgs {
    /// Set a custom path for the kantui configuration file.
    #[clap(short, long, default_value_t=String::from("/etc/kantui/kantui_conf.toml"))]
    config_file_path: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct AppConfig {
    pub stop_timeout: u8,
    pub container_list_refresh_fps: u8,
    pub log_tail_lines: u32,
    pub socket_path: String,
    pub keyconfig: KeyConfig,
}

// Since cursive treats Ctrl+Char, Alt+Char and Char as different variants
// of an enum that is not directly serializable, and we want to hide as much
// implementation details from the config file, a custom KbdEvent wrapper struct
// is implemented.
// It can be easily converted to the cursive event type
// and it implements custom from-string deserialization logic that handles the
// above mentioned use-cases.
#[derive(Debug, Clone)]
pub struct KbdEvent {
    event: event::Event,
}

impl Into<event::Event> for KbdEvent {
    fn into(self) -> event::Event {
        self.event
    }
}

impl Display for KbdEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str_repr = match self.event {
            event::Event::Char(c) => String::from(c),
            event::Event::CtrlChar(c) => format!("{CTRL_REPR}{c}"),
            event::Event::AltChar(c) => format!("{ALT_REPR}{c}"),
            _ => String::new(),
        };
        write!(f, "{}", str_repr)
    }
}

impl Serialize for KbdEvent {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for KbdEvent {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let s: Vec<char> = String::deserialize(deserializer)?
            .chars()
            .filter(|c| c.is_ascii() && !c.is_whitespace())
            .collect();

        let first_char = s.get(0).ok_or(de::Error::custom(
            "No first character specified for key binding",
        ))?;

        let event = match *first_char {
            CTRL_REPR => event::Event::CtrlChar(*s.get(1).ok_or(de::Error::custom(format!(
                "No second character specified for key binding after {CTRL_REPR}"
            )))?),
            ALT_REPR => event::Event::AltChar(*s.get(1).ok_or(de::Error::custom(format!(
                "No second character specified for key binding after {ALT_REPR}"
            )))?),
            _ => event::Event::Char(*first_char),
        };

        Ok(KbdEvent { event })
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct KeyConfig {
    pub start_btn_name: String,
    pub start_kbd_key: KbdEvent,

    pub stop_btn_name: String,
    pub stop_kbd_key: KbdEvent,

    pub remove_btn_name: String,
    pub remove_kbd_key: KbdEvent,

    pub logs_btn_name: String,
    pub logs_kbd_key: KbdEvent,

    pub help_btn_name: String,
    pub help_kbd_key: KbdEvent,

    pub quit_btn_name: String,
    pub quit_kbd_key: KbdEvent,

    pub describe_btn_name: String,
    pub describe_kbd_key: KbdEvent,

    pub redeploy_btn_name: String,
    pub redeploy_kbd_key: KbdEvent,
    pub redeploy_command: String,
}

fn parse_conf_file(conf_file_path: &str) -> Result<AppConfig> {
    let app_conf_builder = Config::builder()
        .add_source(config::File::with_name(conf_file_path))
        // Add in/override settings from the environment (with a prefix of KANTUI)
        // E.g. `KANTUI_STOP_TIMEOUT=5 kantui` would set the `timeout` value to 5
        // These override config-file defined variables
        .add_source(config::Environment::with_prefix("KANTUI"))
        .build()?;

    match app_conf_builder.try_deserialize::<AppConfig>() {
        Ok(config) => Ok(config),
        Err(e) => Err(Box::new(e)),
    }
}

fn parse_cli() -> CliArgs {
    let args = CliArgs::parse();

    if !PathBuf::from(&args.config_file_path).exists() {
        eprintln!("Config file at: {} does not exist", &args.config_file_path);
        std::process::exit(-1);
    }
    args
}

pub fn get_app_configuration() -> Result<AppConfig> {
    let cli_args = parse_cli();
    parse_conf_file(&cli_args.config_file_path)
}
