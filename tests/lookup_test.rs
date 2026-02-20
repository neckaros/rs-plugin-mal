use extism::*;
use rs_plugin_common_interfaces::{
    domain::rs_ids::RsIds,
    lookup::{RsLookupQuery, RsLookupSerie, RsLookupWrapper},
    CredentialType, PluginCredential,
};
use std::sync::Once;

static LOAD_TEST_ENV: Once = Once::new();

fn load_test_env() {
    LOAD_TEST_ENV.call_once(|| {
        let _ = dotenvy::from_filename(".env.test");
    });
}

fn build_plugin() -> Plugin {
    let wasm = Wasm::file("target/wasm32-unknown-unknown/release/rs_plugin_mal.wasm");
    let manifest = Manifest::new([wasm]).with_allowed_host("api.myanimelist.net");
    Plugin::new(&manifest, [], true).expect("Failed to create plugin")
}

fn call_lookup(plugin: &mut Plugin, input: &RsLookupWrapper) -> serde_json::Value {
    let input_str = serde_json::to_string(input).unwrap();
    let output = plugin
        .call::<&str, &[u8]>("lookup_metadata", &input_str)
        .expect("lookup_metadata call failed");
    serde_json::from_slice(output).expect("Failed to parse output JSON")
}

fn call_lookup_images(plugin: &mut Plugin, input: &RsLookupWrapper) -> serde_json::Value {
    let input_str = serde_json::to_string(input).unwrap();
    let output = plugin
        .call::<&str, &[u8]>("lookup_metadata_images", &input_str)
        .expect("lookup_metadata_images call failed");
    serde_json::from_slice(output).expect("Failed to parse output JSON")
}

fn mal_credential() -> Option<PluginCredential> {
    load_test_env();
    let client_id = std::env::var("MAL_CLIENT_ID").ok()?;
    Some(PluginCredential {
        kind: CredentialType::Token,
        login: None,
        password: Some(client_id),
        settings: serde_json::Value::Null,
        user_ref: None,
        refresh_token: None,
        expires: None,
    })
}

#[test]
fn test_lookup_without_credential_returns_401() {
    let mut plugin = build_plugin();

    let input = RsLookupWrapper {
        query: RsLookupQuery::Serie(RsLookupSerie {
            name: Some("One Piece".to_string()),
            ids: None,
        }),
        credential: None,
        params: None,
    };

    let input_str = serde_json::to_string(&input).unwrap();
    let error = plugin
        .call::<&str, &[u8]>("lookup_metadata", &input_str)
        .expect_err("Expected 401 error when client ID credential is missing");
    let message = error.to_string();
    assert!(
        message.contains("No Client ID provided") || message.contains("401"),
        "Expected missing client ID message/401, got: {message}"
    );
}

#[test]
fn test_lookup_one_punch_man_by_name() {
    let Some(credential) = mal_credential() else {
        eprintln!("Skipping test_lookup_one_punch_man_by_name: MAL_CLIENT_ID is not set");
        return;
    };

    let mut plugin = build_plugin();

    let input = RsLookupWrapper {
        query: RsLookupQuery::Serie(RsLookupSerie {
            name: Some("one punch man".to_string()),
            ids: None,
        }),
        credential: Some(credential),
        params: None,
    };

    let results = call_lookup(&mut plugin, &input);
    let results_array = results.as_array().expect("Expected an array");
    assert!(
        !results_array.is_empty(),
        "Expected at least one result for 'one punch man'"
    );

    println!(
        "\n=== One Punch Man search results ({} found) ===",
        results_array.len()
    );
    for (i, result) in results_array.iter().take(1).enumerate() {
        println!("\n--- Result {} ---", i + 1);
        println!("{}", serde_json::to_string_pretty(result).unwrap());
    }
}

#[test]
fn test_lookup_one_piece_by_name() {
    let Some(credential) = mal_credential() else {
        eprintln!("Skipping test_lookup_one_piece_by_name: MAL_CLIENT_ID is not set");
        return;
    };

    let mut plugin = build_plugin();

    let input = RsLookupWrapper {
        query: RsLookupQuery::Serie(RsLookupSerie {
            name: Some("One Piece".to_string()),
            ids: None,
        }),
        credential: Some(credential),
        params: None,
    };

    let results = call_lookup(&mut plugin, &input);
    let results_array = results.as_array().expect("Expected an array");
    assert!(
        !results_array.is_empty(),
        "Expected at least one result for 'One Piece'"
    );

    println!(
        "\n=== One Piece search results ({} found) ===",
        results_array.len()
    );
    for (i, result) in results_array.iter().take(1).enumerate() {
        println!("\n--- Result {} ---", i + 1);
        println!("{}", serde_json::to_string_pretty(result).unwrap());
    }
}

#[test]
fn test_lookup_one_piece_by_mal_id() {
    let Some(credential) = mal_credential() else {
        eprintln!("Skipping test_lookup_one_piece_by_mal_id: MAL_CLIENT_ID is not set");
        return;
    };

    let mut plugin = build_plugin();

    let input = RsLookupWrapper {
        query: RsLookupQuery::Serie(RsLookupSerie {
            name: None,
            ids: Some(RsIds {
                myanimelist_manga_id: Some(21),
                ..Default::default()
            }),
        }),
        credential: Some(credential),
        params: None,
    };

    let results = call_lookup(&mut plugin, &input);
    let results_array = results.as_array().expect("Expected an array");
    assert_eq!(
        results_array.len(),
        1,
        "Expected exactly one result when fetching by ID"
    );

    let serie = &results_array[0]["metadata"]["serie"];
    assert_eq!(serie["id"], "mal:21");

    println!(
        "\n=== One Piece by ID result ===\n{}",
        serde_json::to_string_pretty(&results_array[0]).unwrap()
    );
}

#[test]
fn test_lookup_empty_name_returns_404() {
    let Some(credential) = mal_credential() else {
        eprintln!("Skipping test_lookup_empty_name_returns_404: MAL_CLIENT_ID is not set");
        return;
    };

    let mut plugin = build_plugin();

    let input = RsLookupWrapper {
        query: RsLookupQuery::Serie(RsLookupSerie {
            name: Some("".to_string()),
            ids: None,
        }),
        credential: Some(credential),
        params: None,
    };

    let input_str = serde_json::to_string(&input).unwrap();
    let error = plugin
        .call::<&str, &[u8]>("lookup_metadata", &input_str)
        .expect_err("Expected 404 error for empty search");
    let message = error.to_string();
    assert!(
        message.contains("Not supported") || message.contains("404"),
        "Expected error message to mention 404/Not supported, got: {message}"
    );
}

#[test]
fn test_lookup_images_by_mal_id() {
    let Some(credential) = mal_credential() else {
        eprintln!("Skipping test_lookup_images_by_mal_id: MAL_CLIENT_ID is not set");
        return;
    };

    let mut plugin = build_plugin();

    let input = RsLookupWrapper {
        query: RsLookupQuery::Serie(RsLookupSerie {
            name: None,
            ids: Some(RsIds {
                myanimelist_manga_id: Some(21),
                ..Default::default()
            }),
        }),
        credential: Some(credential),
        params: None,
    };

    let images = call_lookup_images(&mut plugin, &input);
    let images_array = images.as_array().expect("Expected an array");
    assert!(
        !images_array.is_empty(),
        "Expected at least one image when fetching by ID"
    );
}
