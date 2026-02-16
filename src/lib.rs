use extism_pdk::{http, log, plugin_fn, FnResult, HttpRequest, Json, LogLevel, WithReturnCode};

use rs_plugin_common_interfaces::{
    domain::external_images::ExternalImage,
    lookup::{RsLookupMetadataResultWithImages, RsLookupQuery, RsLookupWrapper},
    CredentialType, CustomParam, CustomParamTypes, PluginInformation, PluginType,
};

mod convert;
mod mal;

use convert::{mal_anime_to_images, mal_anime_to_result};
use mal::{
    build_id_images_url, build_id_url, build_search_images_url, build_search_url, MyAnimeListAnime,
    MyAnimeListSearchResponse,
};

#[plugin_fn]
pub fn infos() -> FnResult<Json<PluginInformation>> {
    Ok(Json(PluginInformation {
        name: "myanimelist_metadata".into(),
        capabilities: vec![PluginType::LookupMetadata],
        version: 2,
        interface_version: 1,
        repo: None,
        publisher: "neckaros".into(),
        description: "Look up anime metadata from MyAnimeList".into(),
        credential_kind: Some(CredentialType::Token),
        settings: vec![CustomParam {
            name: "allow_nsfw_content".to_string(),
            param: CustomParamTypes::Text(Some("false".to_string())),
            description: Some("Allow NSFW content (true/false)".to_string()),
            required: false,
        }],
        ..Default::default()
    }))
}

fn extract_mal_id(query: &RsLookupQuery) -> Option<u64> {
    match query {
        RsLookupQuery::Serie(s) => s.ids.as_ref().and_then(|ids| ids.myanimelist_manga_id),
        RsLookupQuery::Movie(m) => m.ids.as_ref().and_then(|ids| ids.myanimelist_manga_id),
        _ => None,
    }
}

fn extract_client_id(lookup: &RsLookupWrapper) -> Option<String> {
    let from_credential = lookup.credential.as_ref().and_then(|credential| {
        credential
            .password
            .clone()
            .or_else(|| credential.login.clone())
    });

    from_credential.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn parse_bool(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Some(true),
        "false" | "0" | "no" | "off" => Some(false),
        _ => None,
    }
}

fn extract_allow_nsfw(lookup: &RsLookupWrapper) -> bool {
    lookup
        .params
        .as_ref()
        .and_then(|params| {
            [
                "allow_nsfw_content",
                "allowNsfwContent",
                "Allow NSFW content",
            ]
            .iter()
            .find_map(|key| params.get(*key))
        })
        .and_then(|value| parse_bool(value))
        .unwrap_or(false)
}

fn build_http_request(url: String, client_id: &str) -> HttpRequest {
    let mut request = HttpRequest {
        url,
        headers: Default::default(),
        method: Some("GET".into()),
    };

    request
        .headers
        .insert("Accept".to_string(), "application/json".to_string());
    request
        .headers
        .insert("X-MAL-CLIENT-ID".to_string(), client_id.to_string());

    request
}

fn fetch_by_id(id: u64, client_id: &str) -> FnResult<Vec<MyAnimeListAnime>> {
    execute_id_request(build_id_url(id), client_id)
}

fn fetch_by_search(
    search: &str,
    client_id: &str,
    allow_nsfw: bool,
) -> FnResult<Vec<MyAnimeListAnime>> {
    execute_search_request(build_search_url(search, allow_nsfw), client_id)
}

fn fetch_images_by_id(id: u64, client_id: &str) -> FnResult<Vec<MyAnimeListAnime>> {
    execute_id_request(build_id_images_url(id), client_id)
}

fn fetch_images_by_search(
    search: &str,
    client_id: &str,
    allow_nsfw: bool,
) -> FnResult<Vec<MyAnimeListAnime>> {
    execute_search_request(build_search_images_url(search, allow_nsfw), client_id)
}

fn execute_id_request(url: String, client_id: &str) -> FnResult<Vec<MyAnimeListAnime>> {
    let request = build_http_request(url, client_id);
    let res = http::request::<Vec<u8>>(&request, None);

    match res {
        Ok(res) if res.status_code() >= 200 && res.status_code() < 300 => {
            match res.json::<MyAnimeListAnime>() {
                Ok(anime) => Ok(vec![anime]),
                Err(e) => {
                    log!(LogLevel::Error, "MAL JSON parse error: {}", e);
                    Err(WithReturnCode::new(e, 500))
                }
            }
        }
        Ok(res) => {
            log!(
                LogLevel::Error,
                "MAL HTTP error {}: {}",
                res.status_code(),
                String::from_utf8_lossy(&res.body())
            );
            Err(WithReturnCode::new(
                extism_pdk::Error::msg(format!("HTTP error: {}", res.status_code())),
                res.status_code() as i32,
            ))
        }
        Err(e) => {
            log!(LogLevel::Error, "MAL request failed: {}", e);
            Err(WithReturnCode(e, 500))
        }
    }
}

fn execute_search_request(url: String, client_id: &str) -> FnResult<Vec<MyAnimeListAnime>> {
    let request = build_http_request(url, client_id);
    let res = http::request::<Vec<u8>>(&request, None);

    match res {
        Ok(res) if res.status_code() >= 200 && res.status_code() < 300 => {
            match res.json::<MyAnimeListSearchResponse>() {
                Ok(response) => Ok(response.data.into_iter().map(|item| item.node).collect()),
                Err(e) => {
                    log!(LogLevel::Error, "MAL JSON parse error: {}", e);
                    Err(WithReturnCode::new(e, 500))
                }
            }
        }
        Ok(res) => {
            log!(
                LogLevel::Error,
                "MAL HTTP error {}: {}",
                res.status_code(),
                String::from_utf8_lossy(&res.body())
            );
            Err(WithReturnCode::new(
                extism_pdk::Error::msg(format!("HTTP error: {}", res.status_code())),
                res.status_code() as i32,
            ))
        }
        Err(e) => {
            log!(LogLevel::Error, "MAL request failed: {}", e);
            Err(WithReturnCode(e, 500))
        }
    }
}

#[plugin_fn]
pub fn lookup_metadata(
    Json(lookup): Json<RsLookupWrapper>,
) -> FnResult<Json<Vec<RsLookupMetadataResultWithImages>>> {
    let all_media = lookup_media(&lookup)?;

    let results: Vec<RsLookupMetadataResultWithImages> =
        all_media.into_iter().map(mal_anime_to_result).collect();

    Ok(Json(results))
}

fn lookup_media(lookup: &RsLookupWrapper) -> FnResult<Vec<MyAnimeListAnime>> {
    lookup_media_with_fetchers(lookup, fetch_by_id, fetch_by_search)
}

fn lookup_media_images(lookup: &RsLookupWrapper) -> FnResult<Vec<MyAnimeListAnime>> {
    lookup_media_with_fetchers(lookup, fetch_images_by_id, fetch_images_by_search)
}

fn lookup_media_with_fetchers(
    lookup: &RsLookupWrapper,
    fetch_by_id_fn: fn(u64, &str) -> FnResult<Vec<MyAnimeListAnime>>,
    fetch_by_search_fn: fn(&str, &str, bool) -> FnResult<Vec<MyAnimeListAnime>>,
) -> FnResult<Vec<MyAnimeListAnime>> {
    match &lookup.query {
        RsLookupQuery::Serie(_) | RsLookupQuery::Movie(_) => {}
        _ => return Ok(vec![]),
    }

    let client_id = extract_client_id(lookup)
        .ok_or_else(|| WithReturnCode::new(extism_pdk::Error::msg("No Client ID provided"), 401))?;
    let allow_nsfw = extract_allow_nsfw(lookup);

    let all_media = if let Some(mal_id) = extract_mal_id(&lookup.query) {
        fetch_by_id_fn(mal_id, &client_id)?
    } else {
        let search = match &lookup.query {
            RsLookupQuery::Serie(s) => s.name.as_deref(),
            RsLookupQuery::Movie(m) => m.name.as_deref(),
            _ => unreachable!(),
        };

        match search {
            Some(s) if !s.trim().is_empty() => fetch_by_search_fn(s, &client_id, allow_nsfw)?,
            _ => {
                return Err(WithReturnCode::new(
                    extism_pdk::Error::msg("Not supported"),
                    404,
                ));
            }
        }
    };

    Ok(all_media)
}

#[plugin_fn]
pub fn lookup_metadata_images(
    Json(lookup): Json<RsLookupWrapper>,
) -> FnResult<Json<Vec<ExternalImage>>> {
    let all_media = lookup_media_images(&lookup)?;

    let images: Vec<ExternalImage> = all_media
        .into_iter()
        .flat_map(|media| mal_anime_to_images(&media))
        .collect();

    Ok(Json(images))
}
