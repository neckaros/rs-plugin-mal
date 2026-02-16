use rs_plugin_common_interfaces::{
    RsRequest, domain::{
        external_images::{ExternalImage, ImageType},
        serie::{Serie, SerieStatus, SerieType},
    }, lookup::{RsLookupMetadataResult, RsLookupMetadataResultWithImages}
};
use serde_json::json;

use crate::mal::{MyAnimeListAlternativeTitles, MyAnimeListAnime, MyAnimeListPicture};

fn best_title(media: &MyAnimeListAnime) -> String {
    media
        .alternative_titles
        .as_ref()
        .and_then(|titles| titles.en.clone())
        .filter(|title| !title.trim().is_empty())
        .unwrap_or_else(|| media.title.clone())
}

fn push_unique(values: &mut Vec<String>, value: Option<&String>, primary: &str) {
    if let Some(candidate) = value {
        if !candidate.trim().is_empty() && candidate != primary && !values.contains(candidate) {
            values.push(candidate.clone());
        }
    }
}

fn alt_names(media: &MyAnimeListAnime) -> Option<Vec<String>> {
    let primary = best_title(media);
    let mut alts: Vec<String> = Vec::new();

    if media.title != primary {
        alts.push(media.title.clone());
    }

    if let Some(titles) = &media.alternative_titles {
        push_unique(&mut alts, titles.en.as_ref(), &primary);
        push_unique(&mut alts, titles.ja.as_ref(), &primary);

        for synonym in &titles.synonyms {
            if !synonym.trim().is_empty() && synonym != &primary && !alts.contains(synonym) {
                alts.push(synonym.clone());
            }
        }
    }

    if alts.is_empty() {
        None
    } else {
        Some(alts)
    }
}

fn map_status(status: &Option<String>) -> Option<SerieStatus> {
    status.as_ref().map(|s| match s.as_str() {
        "finished_airing" => SerieStatus::Ended,
        "currently_airing" => SerieStatus::Returning,
        "not_yet_aired" => SerieStatus::Planned,
        _ => SerieStatus::Unknown,
    })
}

fn year_from_start_date(start_date: &Option<String>) -> Option<u16> {
    start_date
        .as_deref()
        .and_then(|value| value.split('-').next())
        .and_then(|year| year.parse::<u16>().ok())
}

fn push_image(
    images: &mut Vec<ExternalImage>,
    seen_urls: &mut Vec<String>,
    kind: ImageType,
    url: Option<&String>,
) {
    if let Some(url) = url {
        if !seen_urls.contains(url) {
            seen_urls.push(url.clone());
            images.push(ExternalImage {
                kind: Some(kind),
                url: RsRequest {
                    url: url.clone(),
                    ..Default::default()
                },
                ..Default::default()
            });
        }
    }
}

fn pick_url(picture: &MyAnimeListPicture) -> Option<&String> {
    picture.large.as_ref().or(picture.medium.as_ref())
}

fn build_images(media: &MyAnimeListAnime) -> Vec<ExternalImage> {
    let mut images = Vec::new();
    let mut seen_urls = Vec::new();

    if let Some(main_picture) = &media.main_picture {
        push_image(
            &mut images,
            &mut seen_urls,
            ImageType::Poster,
            pick_url(main_picture),
        );
    }

    if let Some(pictures) = &media.pictures {
        for picture in pictures {
            push_image(
                &mut images,
                &mut seen_urls,
                ImageType::Background,
                pick_url(picture),
            );
        }
    }

    images
}

fn collect_genre_names(titles: &Option<Vec<crate::mal::MyAnimeListGenre>>) -> Option<Vec<String>> {
    titles.as_ref().map(|items| {
        items
            .iter()
            .map(|genre| genre.name.clone())
            .collect::<Vec<_>>()
    })
}

fn build_params(media: &MyAnimeListAnime) -> serde_json::Value {
    let mut params = serde_json::Map::new();

    params.insert("mal_id".to_string(), json!(media.id));

    if let Some(desc) = &media.synopsis {
        params.insert("overview".to_string(), json!(desc));
    }
    if let Some(background) = &media.background {
        params.insert("background".to_string(), json!(background));
    }
    if let Some(genres) = collect_genre_names(&media.genres) {
        params.insert("genres".to_string(), json!(genres));
    }
    if let Some(media_type) = &media.media_type {
        params.insert("mediaType".to_string(), json!(media_type));
    }
    if let Some(episodes) = media.num_episodes {
        params.insert("episodes".to_string(), json!(episodes));
    }
    if let Some(score) = media.mean {
        params.insert("mean".to_string(), json!(score));
    }
    if let Some(popularity) = media.popularity {
        params.insert("popularity".to_string(), json!(popularity));
    }
    if let Some(nsfw) = &media.nsfw {
        params.insert("nsfw".to_string(), json!(nsfw));
    }
    if let Some(rating) = &media.rating {
        params.insert("rating".to_string(), json!(rating));
    }
    if let Some(start_date) = &media.start_date {
        params.insert("startDate".to_string(), json!(start_date));
    }

    serde_json::Value::Object(params)
}

pub fn mal_anime_to_result(media: MyAnimeListAnime) -> RsLookupMetadataResultWithImages {
    let images = build_images(&media);

    let serie = Serie {
        id: format!("mal:{}", media.id),
        name: best_title(&media),
        kind: media.media_type.clone().map(|f| SerieType::from_string(&f)),
        alt: alt_names(&media),
        status: map_status(&media.status),
        year: year_from_start_date(&media.start_date),
        trailer: media.trailer_url.clone(),
        anilist_manga_id: None,
        myanimelist_manga_id: Some(media.id),
        params: Some(build_params(&media)),
        ..Default::default()
    };

    RsLookupMetadataResultWithImages {
        metadata: RsLookupMetadataResult::Serie(serie),
        images,
        ..Default::default()
    }
}

pub fn mal_anime_to_images(media: &MyAnimeListAnime) -> Vec<ExternalImage> {
    build_images(media)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_titles() -> MyAnimeListAlternativeTitles {
        MyAnimeListAlternativeTitles {
            synonyms: vec!["OP".to_string()],
            en: Some("One Piece".to_string()),
            ja: Some("Wan Pisu".to_string()),
        }
    }

    fn sample_picture(url: &str) -> MyAnimeListPicture {
        MyAnimeListPicture {
            medium: None,
            large: Some(url.to_string()),
        }
    }

    fn sample_media() -> MyAnimeListAnime {
        MyAnimeListAnime {
            id: 21,
            title: "ONE PIECE".to_string(),
            main_picture: Some(sample_picture("https://cdn.myanimelist.net/poster.jpg")),
            alternative_titles: Some(sample_titles()),
            start_date: Some("1999-10-20".to_string()),
            synopsis: Some("A pirate adventure.".to_string()),
            mean: Some(8.7),
            popularity: Some(10),
            num_episodes: Some(1000),
            status: Some("currently_airing".to_string()),
            genres: Some(vec![crate::mal::MyAnimeListGenre {
                name: "Adventure".to_string(),
            }]),
            media_type: Some("tv".to_string()),
            rating: Some("pg_13".to_string()),
            nsfw: Some("white".to_string()),
            pictures: Some(vec![sample_picture(
                "https://cdn.myanimelist.net/background.jpg",
            )]),
            trailer_url: Some("https://www.youtube.com/watch?v=example".to_string()),
            background: Some("Some background".to_string()),
        }
    }

    #[test]
    fn test_best_title_prefers_english_title() {
        let media = sample_media();
        assert_eq!(best_title(&media), "One Piece");
    }

    #[test]
    fn test_alt_names_excludes_primary() {
        let media = sample_media();
        let alts = alt_names(&media).unwrap();
        assert!(!alts.contains(&"One Piece".to_string()));
        assert!(alts.contains(&"ONE PIECE".to_string()));
        assert!(alts.contains(&"Wan Pisu".to_string()));
        assert!(alts.contains(&"OP".to_string()));
    }

    #[test]
    fn test_status_mapping() {
        assert_eq!(
            map_status(&Some("finished_airing".to_string())),
            Some(SerieStatus::Ended)
        );
        assert_eq!(
            map_status(&Some("currently_airing".to_string())),
            Some(SerieStatus::Returning)
        );
        assert_eq!(
            map_status(&Some("not_yet_aired".to_string())),
            Some(SerieStatus::Planned)
        );
        assert_eq!(map_status(&None), None);
    }

    #[test]
    fn test_images_include_poster_and_backgrounds() {
        let media = sample_media();
        let images = build_images(&media);

        assert_eq!(images.len(), 2);
        assert_eq!(images[0].kind, Some(ImageType::Poster));
        assert_eq!(images[1].kind, Some(ImageType::Background));
    }

    #[test]
    fn test_year_from_start_date() {
        let media = sample_media();
        assert_eq!(year_from_start_date(&media.start_date), Some(1999));
    }
}
