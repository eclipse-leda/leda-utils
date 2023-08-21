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
use crate::CliArgs;
use anyhow::{anyhow, Result};
use lazy_static::lazy_static;
use rumqttc::{self, Client, Event::Incoming, MqttOptions, Packet::Publish, QoS};
use serde::{self, Deserialize, Serialize};
use serde_json;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

static SERVICE_ID: &str = "kanto_auto_deployer";
// We let CUA take over when it has identified what it should do
static VUM_STATUS_IDENTIFIED: &str = "IDENTIFIED";
static RECONNECT_TIMEOUT: u64 = 2;

lazy_static! {
    static ref LOCK_PATH: PathBuf = {
        match std::option_env!("KAD_LOCK_PATH") {
            Some(p) => PathBuf::from(p),
            None => PathBuf::from("/var/lib/kanto-auto-deployer/KAD.enabled"),
        }
    };
}

#[derive(Serialize, Deserialize, Debug)]
struct VUMEnvelope<T> {
    #[serde(alias = "activityId")]
    activity_id: String,
    timestamp: u64,
    payload: T,
}

#[derive(Serialize, Deserialize, Debug)]
struct FeedbackPayload {
    status: String,
    // We don't care about the rest of the fields
}

type FeedbackMsg = VUMEnvelope<FeedbackPayload>;

fn kad_enabled(lock: &Path) -> bool {
    lock.exists() && lock.is_file()
}

fn disable_kad(lock_path: &Path) -> Result<()> {
    let mut disabled_lock_path = lock_path.to_path_buf();

    if !kad_enabled(lock_path) {
        return Err(anyhow!(
            "Lock {:?} is not a regular file or does not exist",
            lock_path
        ));
    }
    if !disabled_lock_path.set_extension("disabled") {
        return Err(anyhow!(
            "Could not change extension of {:?} to *.disabled",
            lock_path
        ));
    }
    fs::rename(lock_path, disabled_lock_path)?;

    Ok(())
}

fn handle_mqtt_payload(
    payload: &[u8],
    lock_path: &Path,
    thread_terminate_flag: &AtomicBool,
) -> Result<()> {
    // Listen when VUM starts "identifying" what actions it should take.
    let terminate_flag_mqtt = serde_json::from_slice::<FeedbackMsg>(payload)?
        .payload
        .status
        .eq(VUM_STATUS_IDENTIFIED);

    if !terminate_flag_mqtt {
        return Err(anyhow!(
            "Expected status:\"{VUM_STATUS_IDENTIFIED}\" for status"
        ));
    }
    disable_kad(lock_path)?;

    // Will only be reached if everything above was successful
    thread_terminate_flag.store(true, Ordering::Relaxed);
    Ok(())
}

fn try_mqtt_reconnect(timeout: &mut Duration, client: &mut Client, topic: &str, delta: Duration) {
    log::error!(
        "MQTT connection lost, trying to re-subscribe in {} s",
        timeout.as_secs()
    );
    if let Err(e) = client.try_subscribe(topic, QoS::ExactlyOnce) {
        log::debug!("Failed to resubscribe: {e}");
        *timeout += delta;
        std::thread::sleep(*timeout);
    } else {
        // Success, reset timeout
        *timeout = delta;
    }
}

pub fn mqtt_main(cli_config: Arc<CliArgs>, thread_terminate_flag: &AtomicBool) -> Result<()> {
    log::debug!(
        "Trying to start MQTT connection with options {:?}",
        &cli_config.mqtt
    );

    if !kad_enabled(&LOCK_PATH) {
        log::error!(
            "The lock at {:?} does not exist, but KAD was started with the MQTT client \
        option. MQTT listener will not be started, but KAD will still run in daemon mode. \
        If running as a system service this might mean KAD has previously seen the desired \
        state MQTT message and has auto-disabled itself to avoid conflicts with CUA and using \
        it might lead to unexpected behavior.",
            *LOCK_PATH
        );
        return Ok(());
    }
    log::info!("MQTT for daemon mode enabled. Will auto-disable whenever VUM takes over.");

    let mut mqttoptions = MqttOptions::new(SERVICE_ID, &cli_config.mqtt.ip, cli_config.mqtt.port);
    mqttoptions.set_keep_alive(Duration::from_secs(5));
    let delta = Duration::from_secs(RECONNECT_TIMEOUT);
    let mut timeout = delta;

    let (mut client, mut connection) = Client::new(mqttoptions.clone(), 10);
    client.subscribe(&cli_config.mqtt.topic, QoS::ExactlyOnce)?;

    for notification in connection.iter() {
        if let Ok(msg) = notification {
            // We only care about incoming messages
            if let Incoming(Publish(pub_msg)) = msg {
                match handle_mqtt_payload(&pub_msg.payload, &LOCK_PATH, thread_terminate_flag) {
                    Err(e) => {
                        // Message with status VUM_STATUS_IDENTIFYING not found, continue listening
                        log::debug!("MQTT payload handling error: {e}")
                    }
                    Ok(_) => return Ok(()), // Desired state message found, exit MQTT thread
                }
            }
        } else {
            try_mqtt_reconnect(&mut timeout, &mut client, &cli_config.mqtt.topic, delta);
        }
    }

    Ok(())
}
