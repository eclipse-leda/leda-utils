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

use anyhow::{anyhow, Result};
use clap::{Args, Parser};
use inquire::{Select, Text};
use lazy_static::lazy_static;
use rumqttc::{mqttbytes::v4::Packet, Client, Event, MqttOptions, QoS};
use serde::{Deserialize, Serialize};
use serde_json::{self, Map, Value};
use std::path::{Path, PathBuf};
use std::time::Duration;
use std::{fmt::Display, fs::File};
use walkdir::WalkDir;

use crate::blueprint_fetchers::Fetcher;

pub mod blueprint_fetchers;

// Makes it easier when building through BitBake recipes to change the default look-up location and enable
// calling with zero arguments from the end user.
// On an Leda Image this would be set to point at the persistent data partition, e.g.
// DEFAULT_BLUEPRINTS_DIR="/data/var/containers/blueprints" in the recipe itself
lazy_static! {
    static ref DEFAULT_BLUEPRINTS_DIR: &'static str = {
        match std::option_env!("DEFAULT_BLUEPRINTS_DIR") {
            Some(p) => p,
            None => "/var/containers/blueprints",
        }
    };
}

#[derive(Serialize, Deserialize, Debug)]
struct BlueprintMetadata {
    name: String,
    description: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Blueprint {
    #[serde(alias = "blueprintMetadata")]
    metadata: BlueprintMetadata,
    #[serde(alias = "activityId")]
    activity_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    timestamp: Option<u64>,
    payload: Map<String, Value>,
}

// The select menu uses fmt from the Display trait to obtain the string
// representation of the type for visualization in the list
impl Display for Blueprint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.metadata.name, self.metadata.description)
    }
}

#[derive(Debug, Args)]
pub struct MQTTconfig {
    /// Hostname/IP to the MQTT broker where the desired state message would be posted
    #[arg(long = "mqtt-broker-host", default_value = "127.0.0.1")]
    host: String,

    /// Port for the MQTT broker
    #[arg(long = "mqtt-broker-port", default_value_t = 1883)]
    port: u16,

    /// Topic on which to publish the blueprint desired state message
    #[arg(long = "mqtt-topic", default_value = "vehicleupdate/desiredstate")]
    topic: String,
}

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct CLIargs {
    /// The directory containing the SDV bluerprints
    #[arg(short = 'd', long = "blueprints-dir", default_value=*DEFAULT_BLUEPRINTS_DIR)]
    blueprints_dir: PathBuf,

    #[arg(short = 'f', long = "fetch-blueprints", default_value_t=false)]
    update_blueprints: bool,

    /// Extension to use when iterating over the files in the blueprints directory
    #[arg(
        short = 'e',
        long = "blueprints-ext",
        default_value = ".blueprint.json"
    )]
    blueprint_extension: String,

    #[clap(flatten)]
    mqtt: MQTTconfig,
}

fn parse_blueprint(blueprint_path: &Path) -> Result<Blueprint> {
    let f = File::open(blueprint_path)?;
    match serde_json::from_reader(f) {
        Ok(blueprint) => Ok(blueprint),
        Err(e) => Err(anyhow!("{}", e)),
    }
}

fn has_extension(p: &walkdir::DirEntry, extension: &str) -> bool {
    p.path().to_str().unwrap().ends_with(extension)
}

fn load_blueprints(dir: &Path, extension: &str) -> Vec<Blueprint> {
    WalkDir::new(dir)
        .min_depth(1)
        .into_iter()
        .filter_entry(|p| has_extension(p, extension))
        .filter_map(Result::ok)
        .map(|blueprint_path| parse_blueprint(blueprint_path.path()))
        .filter_map(Result::ok)
        .collect()
}

fn publish_blueprint(message: &str, mqtt_conf: &MQTTconfig) -> Result<()> {
    let mut mqttoptions = MqttOptions::new("blueprints-selector", &mqtt_conf.host, mqtt_conf.port);
    mqttoptions.set_keep_alive(Duration::from_secs(5));
    let (mut client, mut connection) = Client::new(mqttoptions, 10);
    client.publish(&mqtt_conf.topic, QoS::ExactlyOnce, false, message)?;

    // Spin the event loop and wait for pub completed packet
    for notification in connection.iter() {
        if let Event::Incoming(Packet::PubComp(_msg)) = notification? {
            println!("Succesfully published blueprint.");
            return Ok(());
        }
    }

    Ok(())
}

fn main() -> Result<()> {
    let cli = CLIargs::parse();

    if cli.update_blueprints {
        println!("You have started blueprint-selector in fetch mode. Choose the way you would like to fetch a new/updated blueprint.");
        let fetcher_kind = Select::new("Choose the type of fetcher you would like to use", blueprint_fetchers::FetcherKind::get_variants_list()).prompt()?;
        let uri = Text::new("Enter the uri from which you would like to fetch from").prompt()?;
        let fetcher = Fetcher::new(fetcher_kind, &uri, &cli.blueprints_dir)?;
        fetcher.fetch()?;
        println!("Successfully downloaded!")
    }

    let blueprint_options = load_blueprints(&cli.blueprints_dir, &cli.blueprint_extension);
    let blueprint = Select::new(
        "Choose a SDV blueprint which you would like to deploy:",
        blueprint_options,
    )
    .prompt()?;

    println!(
        "Publishing blueprint \"{}\" on topic \"{}\".",
        blueprint.metadata.name, cli.mqtt.topic
    );
    let message = serde_json::to_string(&blueprint)?;
    publish_blueprint(&message, &cli.mqtt)?;

    Ok(())
}
