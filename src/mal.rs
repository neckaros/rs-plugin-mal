use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct MyAnimeListSearchResponse {
    #[serde(default)]
    pub data: Vec<MyAnimeListSearchItem>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MyAnimeListSearchItem {
    pub node: MyAnimeListAnime,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MyAnimeListAnime {
    pub id: u64,
    pub title: String,
    pub main_picture: Option<MyAnimeListPicture>,
    pub alternative_titles: Option<MyAnimeListAlternativeTitles>,
    pub start_date: Option<String>,
    pub synopsis: Option<String>,
    pub mean: Option<f64>,
    pub popularity: Option<u64>,
    pub num_episodes: Option<u32>,
    pub status: Option<String>,
    pub genres: Option<Vec<MyAnimeListGenre>>,
    pub studios: Option<Vec<MyAnimeListStudio>>,
    pub media_type: Option<String>,
    pub rating: Option<String>,
    pub nsfw: Option<String>,
    pub pictures: Option<Vec<MyAnimeListPicture>>,
    pub trailer_url: Option<String>,
    pub background: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MyAnimeListPicture {
    pub medium: Option<String>,
    pub large: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MyAnimeListAlternativeTitles {
    #[serde(default)]
    pub synonyms: Vec<String>,
    pub en: Option<String>,
    pub ja: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MyAnimeListGenre {
    pub id: Option<u64>,
    pub name: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MyAnimeListStudio {
    pub id: Option<u64>,
    pub name: String,
}

const FULL_FIELDS: &str = "id,title,main_picture,alternative_titles,start_date,synopsis,mean,popularity,num_episodes,status,genres,studios,media_type,rating,nsfw,pictures,trailer_url,background";
const IMAGE_FIELDS: &str = "id,title,main_picture,pictures";

fn encode_query_component(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());

    for b in value.as_bytes() {
        match *b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(*b as char)
            }
            b' ' => encoded.push_str("%20"),
            _ => encoded.push_str(&format!("%{:02X}", b)),
        }
    }

    encoded
}

pub fn build_id_url(id: u64) -> String {
    format!(
        "https://api.myanimelist.net/v2/anime/{id}?fields={fields}",
        fields = FULL_FIELDS
    )
}

pub fn build_search_url(search: &str, allow_nsfw: bool) -> String {
    let nsfw_param = if allow_nsfw { "&nsfw=true" } else { "" };
    format!(
        "https://api.myanimelist.net/v2/anime?q={query}&limit=25{nsfw}&fields={fields}",
        query = encode_query_component(search),
        nsfw = nsfw_param,
        fields = FULL_FIELDS
    )
}

pub fn build_id_images_url(id: u64) -> String {
    format!(
        "https://api.myanimelist.net/v2/anime/{id}?fields={fields}",
        fields = IMAGE_FIELDS
    )
}

pub fn build_search_images_url(search: &str, allow_nsfw: bool) -> String {
    let nsfw_param = if allow_nsfw { "&nsfw=true" } else { "" };
    format!(
        "https://api.myanimelist.net/v2/anime?q={query}&limit=25{nsfw}&fields={fields}",
        query = encode_query_component(search),
        nsfw = nsfw_param,
        fields = IMAGE_FIELDS
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_only_id_url_does_not_include_metadata_fields() {
        let query = build_id_images_url(21);
        assert!(query.contains("main_picture"));
        assert!(query.contains("pictures"));
        assert!(!query.contains("synopsis"));
        assert!(!query.contains("status"));
    }

    #[test]
    fn image_only_search_url_does_not_include_metadata_fields() {
        let query = build_search_images_url("One Piece", false);
        assert!(query.contains("main_picture"));
        assert!(query.contains("pictures"));
        assert!(!query.contains("synopsis"));
        assert!(!query.contains("status"));
        assert!(query.contains("q=One%20Piece"));
        assert!(!query.contains("nsfw=true"));
    }

    #[test]
    fn search_url_includes_nsfw_when_enabled() {
        let query = build_search_url("One Piece", true);
        assert!(query.contains("q=One%20Piece"));
        assert!(query.contains("nsfw=true"));
    }

    #[test]
    fn image_search_url_includes_nsfw_when_enabled() {
        let query = build_search_images_url("One Piece", true);
        assert!(query.contains("q=One%20Piece"));
        assert!(query.contains("nsfw=true"));
    }
}
