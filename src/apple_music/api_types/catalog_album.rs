use serde::Deserialize;

#[derive(Deserialize)]
pub struct Root {
    pub(in crate::apple_music) data: Vec<Album>,
}

#[derive(Deserialize)]
pub struct Album {
    pub(in crate::apple_music) id: String,
    pub(in crate::apple_music) attributes: AlbumAttributes,
    pub(in crate::apple_music) relationships: AlbumRelationshipsWithTracks,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlbumAttributes {
    /// All of the album's artists
    pub(in crate::apple_music) artist_name: String,
    pub(in crate::apple_music) name: String,
    /// YYYY-MM-DD
    pub(in crate::apple_music) release_date: String,
    pub(in crate::apple_music) track_count: u8,
}

#[derive(Deserialize)]
pub struct AlbumRelationshipsWithTracks {
    pub(in crate::apple_music) tracks: AlbumRelationshipsTracks,
}

#[derive(Deserialize)]
pub struct AlbumRelationshipsTracks {
    pub(in crate::apple_music) data: Vec<Song>,
}

#[derive(Deserialize)]
pub struct Song {
    pub(in crate::apple_music) attributes: SongAttributes,
    pub(in crate::apple_music) id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SongAttributes {
    /// All of the album's artists
    pub(in crate::apple_music) artist_name: String,
    pub(in crate::apple_music) content_rating: Option<ContentRating>,
    pub(in crate::apple_music) disc_number: u8,
    pub(in crate::apple_music) isrc: String,
    pub(in crate::apple_music) name: String,
    /// YYYY-MM-DD
    pub(in crate::apple_music) release_date: String,
    pub(in crate::apple_music) track_number: u8,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContentRating {
    Explicit,
}
