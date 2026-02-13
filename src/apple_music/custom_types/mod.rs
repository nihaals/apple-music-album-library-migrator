use std::collections::{HashMap, HashSet};

use anyhow::{Context, Result, ensure};

use crate::apple_music::api_types;

#[derive(Debug, PartialEq, Eq)]
pub struct Album<Track> {
    pub catalog_id: String,
    pub name: String,
    /// All of the album's artists
    pub artist_name: String,
    /// YYYY-MM-DD
    pub release_date: String,
    pub tracks: Vec<Track>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct TrackNoLibrary {
    pub catalog_id: String,
    pub name: String,
    /// All of the album's artists
    pub artist_name: String,
    pub is_explicit: bool,
    pub isrc: String,
    /// YYYY-MM-DD
    pub release_date: String,
}

#[derive(Debug, PartialEq, Eq)]
pub struct TrackWithLibrary {
    pub catalog_id: String,
    pub name: String,
    /// All of the album's artists
    pub artist_name: String,
    pub is_explicit: bool,
    pub isrc: String,
    /// YYYY-MM-DD
    pub release_date: String,

    /// Starts with `i.`
    pub library_id: Option<String>,
}

impl TryFrom<api_types::catalog_album::Root> for Album<TrackNoLibrary> {
    type Error = anyhow::Error;

    fn try_from(value: api_types::catalog_album::Root) -> Result<Self, Self::Error> {
        ensure!(value.data.len() == 1);
        let album = value.data.into_iter().next().unwrap();

        let mut tracks: Vec<(u8, u8, TrackNoLibrary)> = album
            .relationships
            .tracks
            .data
            .into_iter()
            .map(|song| {
                (
                    song.attributes.disc_number,
                    song.attributes.track_number,
                    TrackNoLibrary {
                        catalog_id: song.id,
                        name: song.attributes.name,
                        artist_name: song.attributes.artist_name,
                        is_explicit: song.attributes.content_rating.is_some(),
                        isrc: song.attributes.isrc,
                        release_date: song.attributes.release_date,
                    },
                )
            })
            .collect();

        ensure!(tracks.len() == album.attributes.track_count as usize);

        tracks.sort_by_key(|(disc, num, _)| (*disc, *num));

        {
            // Check for contiguous 1..N track numbers per disc
            let mut current_disc: Option<u8> = None;
            let mut expected_track_number = 1u8;
            for (disc, num, _) in &tracks {
                if Some(*disc) != current_disc {
                    current_disc = Some(*disc);
                    expected_track_number = 1;
                }
                ensure!(*num == expected_track_number);
                expected_track_number = expected_track_number
                    .checked_add(1)
                    .context("Failed to increment expected track number")?;
            }
        }

        let mut seen_ids = HashSet::new();
        for (_, _, track) in &tracks {
            ensure!(seen_ids.insert(&track.catalog_id));
        }

        Ok(Album {
            catalog_id: album.id,
            name: album.attributes.name,
            artist_name: album.attributes.artist_name,
            release_date: album.attributes.release_date,
            tracks: tracks.into_iter().map(|(_, _, t)| t).collect(),
        })
    }
}

impl TrackNoLibrary {
    fn with_library_id(self, library_id: Option<String>) -> TrackWithLibrary {
        TrackWithLibrary {
            catalog_id: self.catalog_id,
            name: self.name,
            artist_name: self.artist_name,
            is_explicit: self.is_explicit,
            isrc: self.isrc,
            release_date: self.release_date,
            library_id,
        }
    }
}

impl Album<TrackNoLibrary> {
    pub fn with_library_info(
        self,
        library_response: &api_types::library_album::Root,
    ) -> Result<Album<TrackWithLibrary>> {
        ensure!(library_response.data.len() == 1);
        let library_album = &library_response.data[0];
        ensure!(library_album.relationships.catalog.data.len() == 1);
        ensure!(library_album.relationships.catalog.data[0].id == self.catalog_id);

        let mut catalog_to_library: HashMap<&str, &str> = HashMap::new();
        for library_song in &library_album.relationships.tracks.data {
            let catalog_id = &library_song.attributes.play_params.catalog_id;
            ensure!(!catalog_to_library.contains_key(catalog_id.as_str()));
            ensure!(self.tracks.iter().any(|t| &t.catalog_id == catalog_id));
            catalog_to_library.insert(catalog_id, &library_song.id);
        }

        ensure!(!catalog_to_library.is_empty());

        let tracks: Vec<TrackWithLibrary> = self
            .tracks
            .into_iter()
            .map(|track| {
                let library_id = catalog_to_library
                    .get(track.catalog_id.as_str())
                    .map(|&id| id.to_owned());
                track.with_library_id(library_id)
            })
            .collect();

        Ok(Album {
            catalog_id: self.catalog_id,
            name: self.name,
            artist_name: self.artist_name,
            release_date: self.release_date,
            tracks,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_catalog_album_into_album_single_track() {
        let response = api_types::catalog_album::Root {
            data: vec![api_types::catalog_album::Album {
                id: "1".to_owned(),
                attributes: api_types::catalog_album::AlbumAttributes {
                    name: "Album".to_owned(),
                    artist_name: "Artist".to_owned(),
                    release_date: "2000-01-01".to_owned(),
                    track_count: 1,
                },
                relationships: api_types::catalog_album::AlbumRelationshipsWithTracks {
                    tracks: api_types::catalog_album::AlbumRelationshipsTracks {
                        data: vec![api_types::catalog_album::Song {
                            id: "1".to_owned(),
                            attributes: api_types::catalog_album::SongAttributes {
                                name: "Song 1".to_owned(),
                                artist_name: "Artist".to_owned(),
                                content_rating: None,
                                disc_number: 1,
                                isrc: "ISRC1".to_owned(),
                                release_date: "2000-01-01".to_owned(),
                                track_number: 1,
                            },
                        }],
                    },
                },
            }],
        };
        let album = Album::try_from(response).unwrap();
        let expected = Album {
            catalog_id: "1".to_owned(),
            name: "Album".to_owned(),
            artist_name: "Artist".to_owned(),
            release_date: "2000-01-01".to_owned(),
            tracks: vec![TrackNoLibrary {
                catalog_id: "1".to_owned(),
                name: "Song 1".to_owned(),
                artist_name: "Artist".to_owned(),
                is_explicit: false,
                isrc: "ISRC1".to_owned(),
                release_date: "2000-01-01".to_owned(),
            }],
        };
        assert_eq!(album, expected);
    }

    #[test]
    fn test_catalog_album_into_album_two_tracks_sorted() {
        let response = api_types::catalog_album::Root {
            data: vec![api_types::catalog_album::Album {
                id: "1".to_owned(),
                attributes: api_types::catalog_album::AlbumAttributes {
                    name: "Album".to_owned(),
                    artist_name: "Artist".to_owned(),
                    release_date: "2000-01-01".to_owned(),
                    track_count: 2,
                },
                relationships: api_types::catalog_album::AlbumRelationshipsWithTracks {
                    tracks: api_types::catalog_album::AlbumRelationshipsTracks {
                        data: vec![
                            api_types::catalog_album::Song {
                                id: "2".to_owned(),
                                attributes: api_types::catalog_album::SongAttributes {
                                    name: "Song 2".to_owned(),
                                    artist_name: "Artist".to_owned(),
                                    content_rating: Some(
                                        api_types::catalog_album::ContentRating::Explicit,
                                    ),
                                    disc_number: 1,
                                    isrc: "ISRC2".to_owned(),
                                    release_date: "2000-01-01".to_owned(),
                                    track_number: 2,
                                },
                            },
                            api_types::catalog_album::Song {
                                id: "1".to_owned(),
                                attributes: api_types::catalog_album::SongAttributes {
                                    name: "Song 1".to_owned(),
                                    artist_name: "Artist".to_owned(),
                                    content_rating: None,
                                    disc_number: 1,
                                    isrc: "ISRC1".to_owned(),
                                    release_date: "2000-01-01".to_owned(),
                                    track_number: 1,
                                },
                            },
                        ],
                    },
                },
            }],
        };
        let album = Album::try_from(response).unwrap();
        let expected = Album {
            catalog_id: "1".to_owned(),
            name: "Album".to_owned(),
            artist_name: "Artist".to_owned(),
            release_date: "2000-01-01".to_owned(),
            tracks: vec![
                TrackNoLibrary {
                    catalog_id: "1".to_owned(),
                    name: "Song 1".to_owned(),
                    artist_name: "Artist".to_owned(),
                    is_explicit: false,
                    isrc: "ISRC1".to_owned(),
                    release_date: "2000-01-01".to_owned(),
                },
                TrackNoLibrary {
                    catalog_id: "2".to_owned(),
                    name: "Song 2".to_owned(),
                    artist_name: "Artist".to_owned(),
                    is_explicit: true,
                    isrc: "ISRC2".to_owned(),
                    release_date: "2000-01-01".to_owned(),
                },
            ],
        };
        assert_eq!(album, expected);
    }

    #[test]
    fn test_catalog_album_into_album_three_tracks_two_discs_sorted() {
        let response = api_types::catalog_album::Root {
            data: vec![api_types::catalog_album::Album {
                id: "1".to_owned(),
                attributes: api_types::catalog_album::AlbumAttributes {
                    name: "Album".to_owned(),
                    artist_name: "Artist".to_owned(),
                    release_date: "2000-01-01".to_owned(),
                    track_count: 3,
                },
                relationships: api_types::catalog_album::AlbumRelationshipsWithTracks {
                    tracks: api_types::catalog_album::AlbumRelationshipsTracks {
                        data: vec![
                            api_types::catalog_album::Song {
                                id: "3".to_owned(),
                                attributes: api_types::catalog_album::SongAttributes {
                                    name: "Song 3".to_owned(),
                                    artist_name: "Artist".to_owned(),
                                    content_rating: None,
                                    disc_number: 2,
                                    isrc: "ISRC3".to_owned(),
                                    release_date: "2000-01-01".to_owned(),
                                    track_number: 2,
                                },
                            },
                            api_types::catalog_album::Song {
                                id: "2".to_owned(),
                                attributes: api_types::catalog_album::SongAttributes {
                                    name: "Song 2".to_owned(),
                                    artist_name: "Artist".to_owned(),
                                    content_rating: Some(
                                        api_types::catalog_album::ContentRating::Explicit,
                                    ),
                                    disc_number: 2,
                                    isrc: "ISRC2".to_owned(),
                                    release_date: "2000-01-01".to_owned(),
                                    track_number: 1,
                                },
                            },
                            api_types::catalog_album::Song {
                                id: "1".to_owned(),
                                attributes: api_types::catalog_album::SongAttributes {
                                    name: "Song 1".to_owned(),
                                    artist_name: "Artist".to_owned(),
                                    content_rating: None,
                                    disc_number: 1,
                                    isrc: "ISRC1".to_owned(),
                                    release_date: "2000-01-01".to_owned(),
                                    track_number: 1,
                                },
                            },
                        ],
                    },
                },
            }],
        };
        let album = Album::try_from(response).unwrap();
        let expected = Album {
            catalog_id: "1".to_owned(),
            name: "Album".to_owned(),
            artist_name: "Artist".to_owned(),
            release_date: "2000-01-01".to_owned(),
            tracks: vec![
                TrackNoLibrary {
                    catalog_id: "1".to_owned(),
                    name: "Song 1".to_owned(),
                    artist_name: "Artist".to_owned(),
                    is_explicit: false,
                    isrc: "ISRC1".to_owned(),
                    release_date: "2000-01-01".to_owned(),
                },
                TrackNoLibrary {
                    catalog_id: "2".to_owned(),
                    name: "Song 2".to_owned(),
                    artist_name: "Artist".to_owned(),
                    is_explicit: true,
                    isrc: "ISRC2".to_owned(),
                    release_date: "2000-01-01".to_owned(),
                },
                TrackNoLibrary {
                    catalog_id: "3".to_owned(),
                    name: "Song 3".to_owned(),
                    artist_name: "Artist".to_owned(),
                    is_explicit: false,
                    isrc: "ISRC3".to_owned(),
                    release_date: "2000-01-01".to_owned(),
                },
            ],
        };
        assert_eq!(album, expected);
    }

    #[test]
    fn test_catalog_album_into_album_track_count_mismatch() {
        let response = api_types::catalog_album::Root {
            data: vec![api_types::catalog_album::Album {
                id: "1".to_owned(),
                attributes: api_types::catalog_album::AlbumAttributes {
                    name: "Album".to_owned(),
                    artist_name: "Artist".to_owned(),
                    release_date: "2000-01-01".to_owned(),
                    track_count: 2,
                },
                relationships: api_types::catalog_album::AlbumRelationshipsWithTracks {
                    tracks: api_types::catalog_album::AlbumRelationshipsTracks {
                        data: vec![api_types::catalog_album::Song {
                            id: "1".to_owned(),
                            attributes: api_types::catalog_album::SongAttributes {
                                name: "Song 1".to_owned(),
                                artist_name: "Artist".to_owned(),
                                content_rating: None,
                                disc_number: 1,
                                isrc: "ISRC1".to_owned(),
                                release_date: "2000-01-01".to_owned(),
                                track_number: 1,
                            },
                        }],
                    },
                },
            }],
        };
        assert!(Album::try_from(response).is_err());
    }

    #[test]
    fn test_catalog_album_into_album_duplicate_track_number() {
        let response = api_types::catalog_album::Root {
            data: vec![api_types::catalog_album::Album {
                id: "1".to_owned(),
                attributes: api_types::catalog_album::AlbumAttributes {
                    name: "Album".to_owned(),
                    artist_name: "Artist".to_owned(),
                    release_date: "2000-01-01".to_owned(),
                    track_count: 2,
                },
                relationships: api_types::catalog_album::AlbumRelationshipsWithTracks {
                    tracks: api_types::catalog_album::AlbumRelationshipsTracks {
                        data: vec![
                            api_types::catalog_album::Song {
                                id: "1".to_owned(),
                                attributes: api_types::catalog_album::SongAttributes {
                                    name: "Song 1".to_owned(),
                                    artist_name: "Artist".to_owned(),
                                    content_rating: None,
                                    disc_number: 1,
                                    isrc: "ISRC1".to_owned(),
                                    release_date: "2000-01-01".to_owned(),
                                    track_number: 1,
                                },
                            },
                            api_types::catalog_album::Song {
                                id: "2".to_owned(),
                                attributes: api_types::catalog_album::SongAttributes {
                                    name: "Song 2".to_owned(),
                                    artist_name: "Artist".to_owned(),
                                    content_rating: None,
                                    disc_number: 1,
                                    isrc: "ISRC2".to_owned(),
                                    release_date: "2000-01-01".to_owned(),
                                    track_number: 1,
                                },
                            },
                        ],
                    },
                },
            }],
        };
        assert!(Album::try_from(response).is_err());
    }

    #[test]
    fn test_catalog_album_into_album_duplicate_track_catalog_id() {
        let response = api_types::catalog_album::Root {
            data: vec![api_types::catalog_album::Album {
                id: "1".to_owned(),
                attributes: api_types::catalog_album::AlbumAttributes {
                    name: "Album".to_owned(),
                    artist_name: "Artist".to_owned(),
                    release_date: "2000-01-01".to_owned(),
                    track_count: 2,
                },
                relationships: api_types::catalog_album::AlbumRelationshipsWithTracks {
                    tracks: api_types::catalog_album::AlbumRelationshipsTracks {
                        data: vec![
                            api_types::catalog_album::Song {
                                id: "1".to_owned(),
                                attributes: api_types::catalog_album::SongAttributes {
                                    name: "Song 1".to_owned(),
                                    artist_name: "Artist".to_owned(),
                                    content_rating: None,
                                    disc_number: 1,
                                    isrc: "ISRC1".to_owned(),
                                    release_date: "2000-01-01".to_owned(),
                                    track_number: 1,
                                },
                            },
                            api_types::catalog_album::Song {
                                id: "1".to_owned(),
                                attributes: api_types::catalog_album::SongAttributes {
                                    name: "Song 2".to_owned(),
                                    artist_name: "Artist".to_owned(),
                                    content_rating: None,
                                    disc_number: 1,
                                    isrc: "ISRC2".to_owned(),
                                    release_date: "2000-01-01".to_owned(),
                                    track_number: 2,
                                },
                            },
                        ],
                    },
                },
            }],
        };
        assert!(Album::try_from(response).is_err());
    }

    #[test]
    fn test_catalog_album_into_album_duplicate_track_catalog_id_across_discs() {
        let response = api_types::catalog_album::Root {
            data: vec![api_types::catalog_album::Album {
                id: "1".to_owned(),
                attributes: api_types::catalog_album::AlbumAttributes {
                    name: "Album".to_owned(),
                    artist_name: "Artist".to_owned(),
                    release_date: "2000-01-01".to_owned(),
                    track_count: 2,
                },
                relationships: api_types::catalog_album::AlbumRelationshipsWithTracks {
                    tracks: api_types::catalog_album::AlbumRelationshipsTracks {
                        data: vec![
                            api_types::catalog_album::Song {
                                id: "1".to_owned(),
                                attributes: api_types::catalog_album::SongAttributes {
                                    name: "Song 1".to_owned(),
                                    artist_name: "Artist".to_owned(),
                                    content_rating: None,
                                    disc_number: 1,
                                    isrc: "ISRC1".to_owned(),
                                    release_date: "2000-01-01".to_owned(),
                                    track_number: 1,
                                },
                            },
                            api_types::catalog_album::Song {
                                id: "1".to_owned(),
                                attributes: api_types::catalog_album::SongAttributes {
                                    name: "Song 2".to_owned(),
                                    artist_name: "Artist".to_owned(),
                                    content_rating: None,
                                    disc_number: 2,
                                    isrc: "ISRC2".to_owned(),
                                    release_date: "2000-01-01".to_owned(),
                                    track_number: 1,
                                },
                            },
                        ],
                    },
                },
            }],
        };
        assert!(Album::try_from(response).is_err());
    }

    #[test]
    fn test_catalog_album_into_album_missing_track_number() {
        let response = api_types::catalog_album::Root {
            data: vec![api_types::catalog_album::Album {
                id: "1".to_owned(),
                attributes: api_types::catalog_album::AlbumAttributes {
                    name: "Album".to_owned(),
                    artist_name: "Artist".to_owned(),
                    release_date: "2000-01-01".to_owned(),
                    track_count: 2,
                },
                relationships: api_types::catalog_album::AlbumRelationshipsWithTracks {
                    tracks: api_types::catalog_album::AlbumRelationshipsTracks {
                        data: vec![
                            api_types::catalog_album::Song {
                                id: "1".to_owned(),
                                attributes: api_types::catalog_album::SongAttributes {
                                    name: "Song 1".to_owned(),
                                    artist_name: "Artist".to_owned(),
                                    content_rating: None,
                                    disc_number: 1,
                                    isrc: "ISRC1".to_owned(),
                                    release_date: "2000-01-01".to_owned(),
                                    track_number: 1,
                                },
                            },
                            api_types::catalog_album::Song {
                                id: "3".to_owned(),
                                attributes: api_types::catalog_album::SongAttributes {
                                    name: "Song 3".to_owned(),
                                    artist_name: "Artist".to_owned(),
                                    content_rating: None,
                                    disc_number: 1,
                                    isrc: "ISRC3".to_owned(),
                                    release_date: "2000-01-01".to_owned(),
                                    track_number: 3,
                                },
                            },
                        ],
                    },
                },
            }],
        };
        assert!(Album::try_from(response).is_err());
    }

    #[test]
    fn test_catalog_album_into_album_missing_track_number_two_discs() {
        let response = api_types::catalog_album::Root {
            data: vec![api_types::catalog_album::Album {
                id: "1".to_owned(),
                attributes: api_types::catalog_album::AlbumAttributes {
                    name: "Album".to_owned(),
                    artist_name: "Artist".to_owned(),
                    release_date: "2000-01-01".to_owned(),
                    track_count: 2,
                },
                relationships: api_types::catalog_album::AlbumRelationshipsWithTracks {
                    tracks: api_types::catalog_album::AlbumRelationshipsTracks {
                        data: vec![
                            api_types::catalog_album::Song {
                                id: "1".to_owned(),
                                attributes: api_types::catalog_album::SongAttributes {
                                    name: "Song 1".to_owned(),
                                    artist_name: "Artist".to_owned(),
                                    content_rating: None,
                                    disc_number: 1,
                                    isrc: "ISRC1".to_owned(),
                                    release_date: "2000-01-01".to_owned(),
                                    track_number: 1,
                                },
                            },
                            api_types::catalog_album::Song {
                                id: "3".to_owned(),
                                attributes: api_types::catalog_album::SongAttributes {
                                    name: "Song 3".to_owned(),
                                    artist_name: "Artist".to_owned(),
                                    content_rating: None,
                                    disc_number: 2,
                                    isrc: "ISRC3".to_owned(),
                                    release_date: "2000-01-01".to_owned(),
                                    track_number: 2,
                                },
                            },
                        ],
                    },
                },
            }],
        };
        assert!(Album::try_from(response).is_err());
    }

    #[test]
    fn test_with_library_info_single_track_added() {
        let album = Album {
            catalog_id: "0".to_owned(),
            name: "Album".to_owned(),
            artist_name: "Artist".to_owned(),
            release_date: "2000-01-01".to_owned(),
            tracks: vec![
                TrackNoLibrary {
                    catalog_id: "1".to_owned(),
                    name: "Song 1".to_owned(),
                    artist_name: "Artist".to_owned(),
                    is_explicit: false,
                    isrc: "ISRC1".to_owned(),
                    release_date: "2000-01-01".to_owned(),
                },
                TrackNoLibrary {
                    catalog_id: "2".to_owned(),
                    name: "Song 2".to_owned(),
                    artist_name: "Artist".to_owned(),
                    is_explicit: false,
                    isrc: "ISRC2".to_owned(),
                    release_date: "2000-01-01".to_owned(),
                },
            ],
        };
        let library_response = api_types::library_album::Root {
            data: vec![api_types::library_album::LibraryAlbum {
                id: "l.0".to_owned(),
                relationships:
                    api_types::library_album::LibraryAlbumRelationshipsWithTracksCatalog {
                        catalog: api_types::library_album::LibraryAlbumRelationshipsCatalog {
                            data: vec![api_types::library_album::LibraryAlbumCatalog {
                                id: "0".to_owned(),
                            }],
                        },
                        tracks: api_types::library_album::LibraryAlbumRelationshipsTracks {
                            data: vec![api_types::library_album::LibrarySong {
                                id: "i.1".to_owned(),
                                attributes: api_types::library_album::LibrarySongAttributes {
                                    play_params: api_types::library_album::LibrarySongPlayParams {
                                        catalog_id: "1".to_owned(),
                                    },
                                },
                            }],
                        },
                    },
            }],
        };
        let expected = Album {
            catalog_id: "0".to_owned(),
            name: "Album".to_owned(),
            artist_name: "Artist".to_owned(),
            release_date: "2000-01-01".to_owned(),
            tracks: vec![
                TrackWithLibrary {
                    catalog_id: "1".to_owned(),
                    name: "Song 1".to_owned(),
                    artist_name: "Artist".to_owned(),
                    is_explicit: false,
                    isrc: "ISRC1".to_owned(),
                    release_date: "2000-01-01".to_owned(),
                    library_id: Some("i.1".to_owned()),
                },
                TrackWithLibrary {
                    catalog_id: "2".to_owned(),
                    name: "Song 2".to_owned(),
                    artist_name: "Artist".to_owned(),
                    is_explicit: false,
                    isrc: "ISRC2".to_owned(),
                    release_date: "2000-01-01".to_owned(),
                    library_id: None,
                },
            ],
        };
        let result = album.with_library_info(&library_response).unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_with_library_info_two_tracks_out_of_order() {
        let album = Album {
            catalog_id: "0".to_owned(),
            name: "Album".to_owned(),
            artist_name: "Artist".to_owned(),
            release_date: "2000-01-01".to_owned(),
            tracks: vec![
                TrackNoLibrary {
                    catalog_id: "1".to_owned(),
                    name: "Song 1".to_owned(),
                    artist_name: "Artist".to_owned(),
                    is_explicit: false,
                    isrc: "ISRC1".to_owned(),
                    release_date: "2000-01-01".to_owned(),
                },
                TrackNoLibrary {
                    catalog_id: "2".to_owned(),
                    name: "Song 2".to_owned(),
                    artist_name: "Artist".to_owned(),
                    is_explicit: false,
                    isrc: "ISRC2".to_owned(),
                    release_date: "2000-01-01".to_owned(),
                },
            ],
        };
        let library_response = api_types::library_album::Root {
            data: vec![api_types::library_album::LibraryAlbum {
                id: "l.0".to_owned(),
                relationships:
                    api_types::library_album::LibraryAlbumRelationshipsWithTracksCatalog {
                        catalog: api_types::library_album::LibraryAlbumRelationshipsCatalog {
                            data: vec![api_types::library_album::LibraryAlbumCatalog {
                                id: "0".to_owned(),
                            }],
                        },
                        tracks: api_types::library_album::LibraryAlbumRelationshipsTracks {
                            data: vec![
                                api_types::library_album::LibrarySong {
                                    id: "i.2".to_owned(),
                                    attributes: api_types::library_album::LibrarySongAttributes {
                                        play_params:
                                            api_types::library_album::LibrarySongPlayParams {
                                                catalog_id: "2".to_owned(),
                                            },
                                    },
                                },
                                api_types::library_album::LibrarySong {
                                    id: "i.1".to_owned(),
                                    attributes: api_types::library_album::LibrarySongAttributes {
                                        play_params:
                                            api_types::library_album::LibrarySongPlayParams {
                                                catalog_id: "1".to_owned(),
                                            },
                                    },
                                },
                            ],
                        },
                    },
            }],
        };
        let expected = Album {
            catalog_id: "0".to_owned(),
            name: "Album".to_owned(),
            artist_name: "Artist".to_owned(),
            release_date: "2000-01-01".to_owned(),
            tracks: vec![
                TrackWithLibrary {
                    catalog_id: "1".to_owned(),
                    name: "Song 1".to_owned(),
                    artist_name: "Artist".to_owned(),
                    is_explicit: false,
                    isrc: "ISRC1".to_owned(),
                    release_date: "2000-01-01".to_owned(),
                    library_id: Some("i.1".to_owned()),
                },
                TrackWithLibrary {
                    catalog_id: "2".to_owned(),
                    name: "Song 2".to_owned(),
                    artist_name: "Artist".to_owned(),
                    is_explicit: false,
                    isrc: "ISRC2".to_owned(),
                    release_date: "2000-01-01".to_owned(),
                    library_id: Some("i.2".to_owned()),
                },
            ],
        };
        let result = album.with_library_info(&library_response).unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_with_library_info_duplicate_tracks() {
        let album = Album {
            catalog_id: "0".to_owned(),
            name: "Album".to_owned(),
            artist_name: "Artist".to_owned(),
            release_date: "2000-01-01".to_owned(),
            tracks: vec![
                TrackNoLibrary {
                    catalog_id: "1".to_owned(),
                    name: "Song 1".to_owned(),
                    artist_name: "Artist".to_owned(),
                    is_explicit: false,
                    isrc: "ISRC1".to_owned(),
                    release_date: "2000-01-01".to_owned(),
                },
                TrackNoLibrary {
                    catalog_id: "2".to_owned(),
                    name: "Song 2".to_owned(),
                    artist_name: "Artist".to_owned(),
                    is_explicit: false,
                    isrc: "ISRC2".to_owned(),
                    release_date: "2000-01-01".to_owned(),
                },
            ],
        };
        let library_response = api_types::library_album::Root {
            data: vec![api_types::library_album::LibraryAlbum {
                id: "l.0".to_owned(),
                relationships:
                    api_types::library_album::LibraryAlbumRelationshipsWithTracksCatalog {
                        catalog: api_types::library_album::LibraryAlbumRelationshipsCatalog {
                            data: vec![api_types::library_album::LibraryAlbumCatalog {
                                id: "0".to_owned(),
                            }],
                        },
                        tracks: api_types::library_album::LibraryAlbumRelationshipsTracks {
                            data: vec![
                                api_types::library_album::LibrarySong {
                                    id: "i.1".to_owned(),
                                    attributes: api_types::library_album::LibrarySongAttributes {
                                        play_params:
                                            api_types::library_album::LibrarySongPlayParams {
                                                catalog_id: "1".to_owned(),
                                            },
                                    },
                                },
                                api_types::library_album::LibrarySong {
                                    id: "i.1".to_owned(),
                                    attributes: api_types::library_album::LibrarySongAttributes {
                                        play_params:
                                            api_types::library_album::LibrarySongPlayParams {
                                                catalog_id: "1".to_owned(),
                                            },
                                    },
                                },
                            ],
                        },
                    },
            }],
        };
        assert!(album.with_library_info(&library_response).is_err());
    }

    #[test]
    fn test_with_library_info_catalog_id_mismatch() {
        let album = Album {
            catalog_id: "0".to_owned(),
            name: "Album".to_owned(),
            artist_name: "Artist".to_owned(),
            release_date: "2000-01-01".to_owned(),
            tracks: vec![TrackNoLibrary {
                catalog_id: "1".to_owned(),
                name: "Song 1".to_owned(),
                artist_name: "Artist".to_owned(),
                is_explicit: false,
                isrc: "ISRC1".to_owned(),
                release_date: "2000-01-01".to_owned(),
            }],
        };
        let library_response = api_types::library_album::Root {
            data: vec![api_types::library_album::LibraryAlbum {
                id: "l.2".to_owned(),
                relationships:
                    api_types::library_album::LibraryAlbumRelationshipsWithTracksCatalog {
                        catalog: api_types::library_album::LibraryAlbumRelationshipsCatalog {
                            data: vec![api_types::library_album::LibraryAlbumCatalog {
                                id: "2".to_owned(),
                            }],
                        },
                        tracks: api_types::library_album::LibraryAlbumRelationshipsTracks {
                            data: vec![api_types::library_album::LibrarySong {
                                id: "i.1".to_owned(),
                                attributes: api_types::library_album::LibrarySongAttributes {
                                    play_params: api_types::library_album::LibrarySongPlayParams {
                                        catalog_id: "1".to_owned(),
                                    },
                                },
                            }],
                        },
                    },
            }],
        };
        assert!(album.with_library_info(&library_response).is_err());
    }

    #[test]
    fn test_with_library_info_unknown_track() {
        let album = Album {
            catalog_id: "0".to_owned(),
            name: "Album".to_owned(),
            artist_name: "Artist".to_owned(),
            release_date: "2000-01-01".to_owned(),
            tracks: vec![TrackNoLibrary {
                catalog_id: "1".to_owned(),
                name: "Song 1".to_owned(),
                artist_name: "Artist".to_owned(),
                is_explicit: false,
                isrc: "ISRC1".to_owned(),
                release_date: "2000-01-01".to_owned(),
            }],
        };
        let library_response = api_types::library_album::Root {
            data: vec![api_types::library_album::LibraryAlbum {
                id: "l.0".to_owned(),
                relationships:
                    api_types::library_album::LibraryAlbumRelationshipsWithTracksCatalog {
                        catalog: api_types::library_album::LibraryAlbumRelationshipsCatalog {
                            data: vec![api_types::library_album::LibraryAlbumCatalog {
                                id: "0".to_owned(),
                            }],
                        },
                        tracks: api_types::library_album::LibraryAlbumRelationshipsTracks {
                            data: vec![api_types::library_album::LibrarySong {
                                id: "i.2".to_owned(),
                                attributes: api_types::library_album::LibrarySongAttributes {
                                    play_params: api_types::library_album::LibrarySongPlayParams {
                                        catalog_id: "2".to_owned(),
                                    },
                                },
                            }],
                        },
                    },
            }],
        };
        assert!(album.with_library_info(&library_response).is_err());
    }

    #[test]
    fn test_with_library_info_no_tracks() {
        let album = Album {
            catalog_id: "0".to_owned(),
            name: "Album".to_owned(),
            artist_name: "Artist".to_owned(),
            release_date: "2000-01-01".to_owned(),
            tracks: vec![TrackNoLibrary {
                catalog_id: "1".to_owned(),
                name: "Song 1".to_owned(),
                artist_name: "Artist".to_owned(),
                is_explicit: false,
                isrc: "ISRC1".to_owned(),
                release_date: "2000-01-01".to_owned(),
            }],
        };
        let library_response = api_types::library_album::Root {
            data: vec![api_types::library_album::LibraryAlbum {
                id: "l.0".to_owned(),
                relationships:
                    api_types::library_album::LibraryAlbumRelationshipsWithTracksCatalog {
                        catalog: api_types::library_album::LibraryAlbumRelationshipsCatalog {
                            data: vec![api_types::library_album::LibraryAlbumCatalog {
                                id: "0".to_owned(),
                            }],
                        },
                        tracks: api_types::library_album::LibraryAlbumRelationshipsTracks {
                            data: vec![],
                        },
                    },
            }],
        };
        assert!(album.with_library_info(&library_response).is_err());
    }

    #[test]
    fn test_with_library_info_no_catalog() {
        let album = Album {
            catalog_id: "0".to_owned(),
            name: "Album".to_owned(),
            artist_name: "Artist".to_owned(),
            release_date: "2000-01-01".to_owned(),
            tracks: vec![TrackNoLibrary {
                catalog_id: "1".to_owned(),
                name: "Song 1".to_owned(),
                artist_name: "Artist".to_owned(),
                is_explicit: false,
                isrc: "ISRC1".to_owned(),
                release_date: "2000-01-01".to_owned(),
            }],
        };
        let library_response = api_types::library_album::Root {
            data: vec![api_types::library_album::LibraryAlbum {
                id: "l.0".to_owned(),
                relationships:
                    api_types::library_album::LibraryAlbumRelationshipsWithTracksCatalog {
                        catalog: api_types::library_album::LibraryAlbumRelationshipsCatalog {
                            data: vec![],
                        },
                        tracks: api_types::library_album::LibraryAlbumRelationshipsTracks {
                            data: vec![api_types::library_album::LibrarySong {
                                id: "i.2".to_owned(),
                                attributes: api_types::library_album::LibrarySongAttributes {
                                    play_params: api_types::library_album::LibrarySongPlayParams {
                                        catalog_id: "2".to_owned(),
                                    },
                                },
                            }],
                        },
                    },
            }],
        };
        assert!(album.with_library_info(&library_response).is_err());
    }
}
