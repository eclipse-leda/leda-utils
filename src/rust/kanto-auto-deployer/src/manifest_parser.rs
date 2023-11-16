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

//! A module for parsing-out container manifests.
//!
//! The only public API is the `try_parse_manifest` function that takes a json string
//! read-out from disk and tries to parse it to the "internal container state representation"
//! for Kanto-CM.
//!
//! If the json is already in the internal state representation it would be parsed out directly.
//! Otherwise an "initdir" style manifest will be assumed and an automatic conversion will be attempted
//! by first expanding-out the manifest (since init-dir style manifests allow missing keys) and re-mapping it
//! to the internal state representation.
use anyhow::anyhow;
use serde_json::{Map, Value};
use crate::containers::github::com::eclipse_kanto::container_management::containerm::api::types::containers::Container;
use json_patch::merge;

/// Takes a key from a "template" and a "data" dictionary and replaces
/// the template's value for that key from the the data dict.
fn update_template(
    template: &mut Map<String, Value>,
    data: &Map<String, Value>,
    template_key: &str,
    data_key: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let template_val = template
        .get_mut(template_key)
        .ok_or_else(|| anyhow!("No such template key \"{}\"", template_key))?;
    let data_val = data
        .get(data_key)
        .ok_or_else(|| anyhow!("No such data key \"{}\"", data_key))?;

    *template_val = data_val.clone();

    Ok(())
}

/// Consume a Result<T, E> variant and print-out a debug log if it's the Err variant
fn try_or_print<T: std::fmt::Debug>(res: Result<T, Box<dyn std::error::Error>>) {
    if let Err(e) = res {
        log::debug!("{:#?}. Using default value...", e)
    }
}

/// Tries to map top-level json properties from kanto container-config style manifests
/// (ref: <https://websites.eclipseprojects.io/kanto/docs/references/containers/container-config/#template>)
/// to kanto internal container state representation by directly cloning their values
/// (check src/kanto_internal_ctr_repr.json.template.in)
fn map_to_internal_state_manifest(
    container_manifest: Value,
) -> Result<Value, Box<dyn std::error::Error>> {
    let int_state_repr = include_str!("kanto_internal_ctr_repr.json.template.in");
    let mut int_state_repr = serde_json::from_str(int_state_repr)?;
    let ctr_manifest: Map<String, Value> = serde_json::from_value(container_manifest)?;

    // These fields are considered mandatory, if they do not exist,
    // fail the manifest re-parsing
    update_template(&mut int_state_repr, &ctr_manifest, "name", "container_name")?;
    update_template(&mut int_state_repr, &ctr_manifest, "image", "image")?;
    // These are considered as configuration options
    // If they are missing, print an error and continue with defaults
    try_or_print(update_template(
        &mut int_state_repr,
        &ctr_manifest,
        "id",
        "container_id",
    ));
    try_or_print(update_template(
        &mut int_state_repr,
        &ctr_manifest,
        "domain_name",
        "domain_name",
    ));
    try_or_print(update_template(
        &mut int_state_repr,
        &ctr_manifest,
        "host_name",
        "host_name",
    ));
    try_or_print(update_template(
        &mut int_state_repr,
        &ctr_manifest,
        "mounts",
        "mount_points",
    ));
    try_or_print(update_template(
        &mut int_state_repr,
        &ctr_manifest,
        "config",
        "config",
    ));
    try_or_print(update_template(
        &mut int_state_repr,
        &ctr_manifest,
        "host_config",
        "host_config",
    ));
    Ok(serde_json::to_value(int_state_repr)?)
}

/// Use the JSON merge patch (RFC IETF 7386) to merge the manifest read from disk with a
/// template containing all available options.
fn expand_container_manifest(manifest: &Value) -> Result<Value, Box<dyn std::error::Error>> {
    let ctr_config_template = include_str!("kanto_internal_ctr_repr.json.template.in");
    let mut ctr_config_template = serde_json::from_str(ctr_config_template)?;
    merge(&mut ctr_config_template, manifest);
    Ok(ctr_config_template)
}

pub fn try_parse_manifest(container_str: &str) -> Result<Container, Box<dyn std::error::Error>> {
    let parsed_json: Container = match serde_json::from_str(container_str) {
        Ok(ctr) => {
            log::debug!("Manifest is in auto-deployer format already. Deploying directly");
            ctr
        }
        Err(_) => {
            log::debug!("Failed to load manifest directly. Will attempt auto-conversion from init-dir format.");
            let manifest = serde_json::from_str(container_str)?;
            let manifest = expand_container_manifest(&manifest)?;
            let internal_state = map_to_internal_state_manifest(manifest)?;

            // pretty-printing is expensive
            if log::log_enabled!(log::Level::Debug) {
                log::debug!(
                    "Obtained: \n {}",
                    serde_json::to_string_pretty(&internal_state)?
                );
            }

            serde_json::from_value(internal_state)?
        }
    };

    if log::log_enabled!(log::Level::Debug) {
        log::debug!(
            "Deploying: \n {}",
            serde_json::to_string_pretty(&parsed_json)?
        );
    }
    Ok(parsed_json)
}
