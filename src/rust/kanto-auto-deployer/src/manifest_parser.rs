use anyhow::anyhow;
use serde_json::{Map, Value};
use crate::containers::github::com::eclipse_kanto::container_management::containerm::api::types::containers::Container;
use json_patch::merge;

fn update_template(
    template: &mut Map<String, Value>,
    data: &Map<String, Value>,
    template_key: &str,
    data_key: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let template_val = template
        .get_mut(template_key)
        .ok_or(anyhow!("No such template key \"{}\"", template_key))?;
    let data_val = data
        .get(data_key)
        .ok_or(anyhow!("No such data key \"{}\"", data_key))?;

    *template_val = data_val.clone();

    Ok(())
}

fn try_or_print<T: std::fmt::Debug>(res: Result<T, Box<dyn std::error::Error>>) {
    match res {
        Err(e) => log::debug!("[PARSER] {:#?}. Using default value...", e),
        _ => (),
    };
}

/// Use the JSON merge patch (RFC IETF 7386) to merge the manifest read from disk with a
/// template containing all available options.
fn expand_container_manifest(manifest: &Value) -> Result<Value, Box<dyn std::error::Error>> {
    let ctr_config_template = include_str!("kanto_internal_ctr_repr.json.template.in");
    let mut ctr_config_template = serde_json::from_str(ctr_config_template)?;
    merge(&mut ctr_config_template, &manifest);
    Ok(ctr_config_template)
}

/// Tries top map top-level json properties from kanto container-config style manifests
/// (ref: https://websites.eclipseprojects.io/kanto/docs/references/containers/container-config/#template)
/// to kanto internal container state representation by directly cloning their values
/// (check src/kanto_internal_ctr_repr.json.template.in)
fn map_to_internal_state_manifest(
    container_manifest: Value,
) -> Result<Value, Box<dyn std::error::Error>> {
    let int_state_repr = include_str!("kanto_internal_ctr_repr.json.template.in");
    let mut int_state_repr: Map<String, Value> = serde_json::from_str(int_state_repr)?;
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

pub fn try_parse_manifests(container_str: &str) -> Result<Container, Box<dyn std::error::Error>> {
    let parsed_json: Container = match serde_json::from_str(&container_str) {
        Ok(ctr) => {
            log::debug!("Manifest is in auto-deployer format already. Deploying directly");
            ctr
        }
        Err(_) => {
            log::warn!("Failed to load manifest directly. Will attempt auto-conversion from init-dir format.");
            let manifest = serde_json::from_str(&container_str)?;
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
