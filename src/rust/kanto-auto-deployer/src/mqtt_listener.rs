use anyhow::{anyhow, Result};
use rumqttc::{self, Client, Event::Incoming, MqttOptions, Packet::Publish, QoS};
use serde_json::{self, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};

use std::time::Duration;

static SERVICE_ID: &str = "KAD";
static TOPIC: &str = "kanto-auto-deployer/state";
static HOST: &str = "localhost";
static PORT: u16 = 1883;
static TERMINATE_KEY_JSON: &str = "terminate";
static RECONNECT_TIMEOUT: u64 = 2;

fn handle_mqtt_payload(payload: &[u8], thread_terminate_flag: &AtomicBool) -> Result<()> {
    // We don't care about non-json messages
    let terminate_flag_mqtt = serde_json::from_slice::<HashMap<String, Value>>(payload)?
        .get(TERMINATE_KEY_JSON)
        .ok_or_else(|| {
            anyhow!("MQTT message is valid json, but does not contain key {TERMINATE_KEY_JSON}")
        })?
        .as_bool()
        .ok_or_else(|| anyhow!("Expected boolean type for value for key {TERMINATE_KEY_JSON}"))?;

    // Only if the termination request is received, update the atomic flag
    if terminate_flag_mqtt {
        thread_terminate_flag.store(true, Ordering::Relaxed);
    }

    Ok(())
}

fn try_mqtt_reconnect(timeout: &mut Duration, client: &mut Client, delta: Duration) {
    println!(
        "MQTT connection lost, trying to re-subscribe in {} s",
        timeout.as_secs()
    );
    if let Err(e) = client.try_subscribe(TOPIC, QoS::ExactlyOnce) {
        println!("Failed to resubscribe: {e}");
        *timeout += delta;
        std::thread::sleep(*timeout);
    } else {
        // Success, reset timeout
        *timeout = delta;
    }
}

pub fn mqtt_main(thread_terminate_flag: &AtomicBool) -> Result<()> {
    let mut mqttoptions = MqttOptions::new(SERVICE_ID, HOST, PORT);
    mqttoptions.set_keep_alive(Duration::from_secs(5));

    let delta = Duration::from_secs(RECONNECT_TIMEOUT);
    let mut timeout = delta;

    let (mut client, mut connection) = Client::new(mqttoptions.clone(), 10);
    client.subscribe(TOPIC, QoS::ExactlyOnce)?;

    for notification in connection.iter() {
        // We only care about incoming messages
        if let Ok(msg) = notification {
            if let Incoming(Publish(pub_msg)) = msg {
                let _r = handle_mqtt_payload(&pub_msg.payload, thread_terminate_flag);
                if let Err(e) = _r {
                    log::debug!("MQTT message parsing error: {e}")
                }
            }
        } else {
            try_mqtt_reconnect(&mut timeout, &mut client, delta);
        }
    }

    Ok(())
}
