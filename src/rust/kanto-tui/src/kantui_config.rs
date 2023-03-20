// /********************************************************************************
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
// ********************************************************************************/
use super::Result;
use clap::Parser;
use config::Config;
use serde::Deserialize;
use std::path::PathBuf;

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
    pub socket_path: String,
    pub stop_timeout: u8,
    pub keyconfig: KeyConfig,
}

#[derive(Clone, Debug, Deserialize)]
pub struct KeyConfig {
    pub start_btn_name: String,
    pub start_kbd_key: char,

    pub stop_btn_name: String,
    pub stop_kbd_key: char,

    pub remove_btn_name: String,
    pub remove_kbd_key: char,

    pub logs_btn_name: String,
    pub logs_kbd_key: char,

    pub quit_btn_name: String,
    pub quit_kbd_key: char,

    pub redeploy_btn_name: String,
    pub redeploy_kbd_key: char,
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
