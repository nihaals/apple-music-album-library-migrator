use std::collections::{HashMap, HashSet};

use anyhow::{Result, ensure};

use crate::apple_music::custom_types::{Album, TrackNoLibrary, TrackWithLibrary};

#[derive(Debug, PartialEq, Eq)]
pub enum TrackMatchResult<'a> {
    Match {
        source: &'a TrackWithLibrary,
        destination: &'a TrackNoLibrary,
    },
    NoMatch {
        source: &'a TrackWithLibrary,
    },
}

pub fn match_tracks<'a>(
    source: &'a Album<TrackWithLibrary>,
    destination: &'a Album<TrackNoLibrary>,
) -> Result<Vec<TrackMatchResult<'a>>> {
    ensure!(
        source.catalog_id != destination.catalog_id,
        "source and destination albums have the same catalog ID: {}",
        source.catalog_id,
    );
    ensure!(!source.tracks.is_empty(), "source album has no tracks");
    ensure!(
        !destination.tracks.is_empty(),
        "destination album has no tracks",
    );

    {
        let mut source_catalog_ids = HashSet::new();
        for track in &source.tracks {
            ensure!(
                source_catalog_ids.insert(&track.catalog_id),
                "duplicate catalog ID in source: {}",
                track.catalog_id,
            );
        }

        let mut destination_catalog_ids = HashSet::new();
        for track in &destination.tracks {
            ensure!(
                destination_catalog_ids.insert(&track.catalog_id),
                "duplicate catalog ID in destination: {}",
                track.catalog_id,
            );
        }

        ensure!(
            source_catalog_ids.is_disjoint(&destination_catalog_ids),
            "source and destination albums have overlapping track catalog IDs",
        );
    }

    {
        let mut source_isrcs = HashSet::new();
        for track in &source.tracks {
            ensure!(
                source_isrcs.insert(&track.isrc),
                "duplicate ISRC in source: {}",
                track.isrc,
            );
        }

        let mut destination_isrcs = HashSet::new();
        for track in &destination.tracks {
            ensure!(
                destination_isrcs.insert(&track.isrc),
                "duplicate ISRC in destination: {}",
                track.isrc,
            );
        }
    }

    let isrc_map: HashMap<&str, usize> = destination
        .tracks
        .iter()
        .enumerate()
        .map(|(i, t)| (t.isrc.as_str(), i))
        .collect();

    let mut name_artist_map: HashMap<(&str, &str), Vec<usize>> = HashMap::new();
    for (i, t) in destination.tracks.iter().enumerate() {
        name_artist_map
            .entry((&t.name, &t.artist_name))
            .or_default()
            .push(i);
    }

    let mut used_destinations: HashSet<usize> = HashSet::new();
    let mut results = Vec::with_capacity(source.tracks.len());

    for source_track in &source.tracks {
        if let Some(&destination_index) = isrc_map.get(source_track.isrc.as_str()) {
            ensure!(used_destinations.insert(destination_index));
            results.push(TrackMatchResult::Match {
                source: source_track,
                destination: &destination.tracks[destination_index],
            });
            continue;
        }

        if let Some(destination_indices) =
            name_artist_map.get(&(&source_track.name, &source_track.artist_name))
        {
            let mut destination_indices = destination_indices.iter();
            let Some(&destination_index) = destination_indices.next() else {
                results.push(TrackMatchResult::NoMatch {
                    source: source_track,
                });
                continue;
            };
            ensure!(
                destination_indices.next().is_none(),
                "ambiguous name and artist match",
            );

            results.push(TrackMatchResult::Match {
                source: source_track,
                destination: &destination.tracks[destination_index],
            });
            continue;
        }

        results.push(TrackMatchResult::NoMatch {
            source: source_track,
        });
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_match_tracks_simple() {
        let source = Album {
            catalog_id: "10".to_owned(),
            name: "Album 1".to_owned(),
            artist_name: "Artist".to_owned(),
            release_date: "2020-01-01".to_owned(),
            tracks: vec![
                TrackWithLibrary {
                    catalog_id: "1".to_owned(),
                    name: "Song 1".to_owned(),
                    artist_name: "Artist".to_owned(),
                    is_explicit: false,
                    isrc: "ISRC1".to_owned(),
                    release_date: "2020-01-01".to_owned(),
                    library_id: None,
                },
                TrackWithLibrary {
                    catalog_id: "2".to_owned(),
                    name: "Song 2".to_owned(),
                    artist_name: "Artist".to_owned(),
                    is_explicit: false,
                    isrc: "ISRC2".to_owned(),
                    release_date: "2020-01-01".to_owned(),
                    library_id: Some("i.2".to_owned()),
                },
            ],
        };
        let destination = Album {
            catalog_id: "11".to_owned(),
            name: "Album 1".to_owned(),
            artist_name: "Artist".to_owned(),
            release_date: "2020-01-02".to_owned(),
            tracks: vec![
                TrackNoLibrary {
                    catalog_id: "3".to_owned(),
                    name: "Song 1".to_owned(),
                    artist_name: "Artist".to_owned(),
                    is_explicit: false,
                    isrc: "ISRC1".to_owned(),
                    release_date: "2020-01-01".to_owned(),
                },
                TrackNoLibrary {
                    catalog_id: "4".to_owned(),
                    name: "Song 2".to_owned(),
                    artist_name: "Artist".to_owned(),
                    is_explicit: false,
                    isrc: "ISRC2".to_owned(),
                    release_date: "2020-01-01".to_owned(),
                },
            ],
        };
        let expected = vec![
            TrackMatchResult::Match {
                source: &source.tracks[0],
                destination: &destination.tracks[0],
            },
            TrackMatchResult::Match {
                source: &source.tracks[1],
                destination: &destination.tracks[1],
            },
        ];
        assert_eq!(match_tracks(&source, &destination).unwrap(), expected);
    }

    #[test]
    fn test_match_tracks_match_source_order() {
        let source = Album {
            catalog_id: "10".to_owned(),
            name: "Album 1".to_owned(),
            artist_name: "Artist".to_owned(),
            release_date: "2020-01-01".to_owned(),
            tracks: vec![
                TrackWithLibrary {
                    catalog_id: "1".to_owned(),
                    name: "Song 1".to_owned(),
                    artist_name: "Artist".to_owned(),
                    is_explicit: false,
                    isrc: "ISRC1".to_owned(),
                    release_date: "2020-01-01".to_owned(),
                    library_id: None,
                },
                TrackWithLibrary {
                    catalog_id: "2".to_owned(),
                    name: "Song 2".to_owned(),
                    artist_name: "Artist".to_owned(),
                    is_explicit: false,
                    isrc: "ISRC2".to_owned(),
                    release_date: "2020-01-01".to_owned(),
                    library_id: Some("i.2".to_owned()),
                },
            ],
        };
        let destination = Album {
            catalog_id: "11".to_owned(),
            name: "Album 1".to_owned(),
            artist_name: "Artist".to_owned(),
            release_date: "2020-01-02".to_owned(),
            tracks: vec![
                TrackNoLibrary {
                    catalog_id: "4".to_owned(),
                    name: "Song 2".to_owned(),
                    artist_name: "Artist".to_owned(),
                    is_explicit: false,
                    isrc: "ISRC2".to_owned(),
                    release_date: "2020-01-01".to_owned(),
                },
                TrackNoLibrary {
                    catalog_id: "3".to_owned(),
                    name: "Song 1".to_owned(),
                    artist_name: "Artist".to_owned(),
                    is_explicit: false,
                    isrc: "ISRC1".to_owned(),
                    release_date: "2020-01-01".to_owned(),
                },
            ],
        };
        let expected = vec![
            TrackMatchResult::Match {
                source: &source.tracks[0],
                destination: &destination.tracks[1],
            },
            TrackMatchResult::Match {
                source: &source.tracks[1],
                destination: &destination.tracks[0],
            },
        ];
        assert_eq!(match_tracks(&source, &destination).unwrap(), expected);
    }

    #[test]
    fn test_match_tracks_prefix_extra_songs() {
        let source = Album {
            catalog_id: "10".to_owned(),
            name: "Album 1".to_owned(),
            artist_name: "Artist".to_owned(),
            release_date: "2020-01-01".to_owned(),
            tracks: vec![TrackWithLibrary {
                catalog_id: "1".to_owned(),
                name: "Song 1".to_owned(),
                artist_name: "Artist".to_owned(),
                is_explicit: false,
                isrc: "ISRC1".to_owned(),
                release_date: "2020-01-01".to_owned(),
                library_id: None,
            }],
        };
        let destination = Album {
            catalog_id: "11".to_owned(),
            name: "Album 1".to_owned(),
            artist_name: "Artist".to_owned(),
            release_date: "2020-01-01".to_owned(),
            tracks: vec![
                TrackNoLibrary {
                    catalog_id: "2".to_owned(),
                    name: "Song 2".to_owned(),
                    artist_name: "Artist".to_owned(),
                    is_explicit: false,
                    isrc: "ISRC2".to_owned(),
                    release_date: "2020-01-01".to_owned(),
                },
                TrackNoLibrary {
                    catalog_id: "3".to_owned(),
                    name: "Song 1".to_owned(),
                    artist_name: "Artist".to_owned(),
                    is_explicit: false,
                    isrc: "ISRC1".to_owned(),
                    release_date: "2020-01-01".to_owned(),
                },
            ],
        };
        let expected = vec![TrackMatchResult::Match {
            source: &source.tracks[0],
            destination: &destination.tracks[1],
        }];
        assert_eq!(match_tracks(&source, &destination).unwrap(), expected);
    }

    #[test]
    fn test_match_tracks_appended_extra_songs() {
        let source = Album {
            catalog_id: "10".to_owned(),
            name: "Album 1".to_owned(),
            artist_name: "Artist".to_owned(),
            release_date: "2020-01-01".to_owned(),
            tracks: vec![TrackWithLibrary {
                catalog_id: "1".to_owned(),
                name: "Song 1".to_owned(),
                artist_name: "Artist".to_owned(),
                is_explicit: false,
                isrc: "ISRC1".to_owned(),
                release_date: "2020-01-01".to_owned(),
                library_id: None,
            }],
        };
        let destination = Album {
            catalog_id: "11".to_owned(),
            name: "Album 1".to_owned(),
            artist_name: "Artist".to_owned(),
            release_date: "2020-01-01".to_owned(),
            tracks: vec![
                TrackNoLibrary {
                    catalog_id: "3".to_owned(),
                    name: "Song 1".to_owned(),
                    artist_name: "Artist".to_owned(),
                    is_explicit: false,
                    isrc: "ISRC1".to_owned(),
                    release_date: "2020-01-01".to_owned(),
                },
                TrackNoLibrary {
                    catalog_id: "4".to_owned(),
                    name: "Song 2".to_owned(),
                    artist_name: "Artist".to_owned(),
                    is_explicit: false,
                    isrc: "ISRC2".to_owned(),
                    release_date: "2020-01-01".to_owned(),
                },
            ],
        };
        let expected = vec![TrackMatchResult::Match {
            source: &source.tracks[0],
            destination: &destination.tracks[0],
        }];
        assert_eq!(match_tracks(&source, &destination).unwrap(), expected);
    }

    #[test]
    fn test_match_tracks_same() {
        let source = Album {
            catalog_id: "10".to_owned(),
            name: "Album 1".to_owned(),
            artist_name: "Artist".to_owned(),
            release_date: "2020-01-01".to_owned(),
            tracks: vec![TrackWithLibrary {
                catalog_id: "1".to_owned(),
                name: "Song 1".to_owned(),
                artist_name: "Artist".to_owned(),
                is_explicit: false,
                isrc: "ISRC1".to_owned(),
                release_date: "2020-01-01".to_owned(),
                library_id: None,
            }],
        };
        let destination = Album {
            catalog_id: "10".to_owned(),
            name: "Album 1".to_owned(),
            artist_name: "Artist".to_owned(),
            release_date: "2020-01-01".to_owned(),
            tracks: vec![TrackNoLibrary {
                catalog_id: "1".to_owned(),
                name: "Song 1".to_owned(),
                artist_name: "Artist".to_owned(),
                is_explicit: false,
                isrc: "ISRC1".to_owned(),
                release_date: "2020-01-01".to_owned(),
            }],
        };
        assert!(match_tracks(&source, &destination).is_err());
    }

    #[test]
    fn test_match_tracks_same_tracks() {
        let source = Album {
            catalog_id: "10".to_owned(),
            name: "Album 1".to_owned(),
            artist_name: "Artist".to_owned(),
            release_date: "2020-01-01".to_owned(),
            tracks: vec![TrackWithLibrary {
                catalog_id: "1".to_owned(),
                name: "Song 1".to_owned(),
                artist_name: "Artist".to_owned(),
                is_explicit: false,
                isrc: "ISRC1".to_owned(),
                release_date: "2020-01-01".to_owned(),
                library_id: None,
            }],
        };
        let destination = Album {
            catalog_id: "11".to_owned(),
            name: "Album 1".to_owned(),
            artist_name: "Artist".to_owned(),
            release_date: "2020-01-01".to_owned(),
            tracks: vec![TrackNoLibrary {
                catalog_id: "1".to_owned(),
                name: "Song 1".to_owned(),
                artist_name: "Artist".to_owned(),
                is_explicit: false,
                isrc: "ISRC1".to_owned(),
                release_date: "2020-01-01".to_owned(),
            }],
        };
        assert!(match_tracks(&source, &destination).is_err());
    }

    #[test]
    fn test_match_tracks_same_album_catalog_id() {
        let source = Album {
            catalog_id: "10".to_owned(),
            name: "Album 1".to_owned(),
            artist_name: "Artist".to_owned(),
            release_date: "2020-01-01".to_owned(),
            tracks: vec![TrackWithLibrary {
                catalog_id: "1".to_owned(),
                name: "Song 1".to_owned(),
                artist_name: "Artist".to_owned(),
                is_explicit: false,
                isrc: "ISRC1".to_owned(),
                release_date: "2020-01-01".to_owned(),
                library_id: None,
            }],
        };
        let destination = Album {
            catalog_id: "10".to_owned(),
            name: "Album 1".to_owned(),
            artist_name: "Artist".to_owned(),
            release_date: "2020-01-02".to_owned(),
            tracks: vec![TrackNoLibrary {
                catalog_id: "2".to_owned(),
                name: "Song 2".to_owned(),
                artist_name: "Artist".to_owned(),
                is_explicit: false,
                isrc: "ISRC2".to_owned(),
                release_date: "2020-01-02".to_owned(),
            }],
        };
        assert!(match_tracks(&source, &destination).is_err());
    }

    #[test]
    fn test_match_tracks_same_title_artist_track() {
        let source = Album {
            catalog_id: "10".to_owned(),
            name: "Album 1".to_owned(),
            artist_name: "Artist".to_owned(),
            release_date: "2020-01-01".to_owned(),
            tracks: vec![TrackWithLibrary {
                catalog_id: "1".to_owned(),
                name: "Song 1".to_owned(),
                artist_name: "Artist".to_owned(),
                is_explicit: false,
                isrc: "ISRC1".to_owned(),
                release_date: "2020-01-01".to_owned(),
                library_id: None,
            }],
        };
        let destination = Album {
            catalog_id: "11".to_owned(),
            name: "Album 1".to_owned(),
            artist_name: "Artist".to_owned(),
            release_date: "2020-01-01".to_owned(),
            tracks: vec![TrackNoLibrary {
                catalog_id: "2".to_owned(),
                name: "Song 1".to_owned(),
                artist_name: "Artist".to_owned(),
                is_explicit: false,
                isrc: "ISRC2".to_owned(),
                release_date: "2020-01-02".to_owned(),
            }],
        };
        let expected = vec![TrackMatchResult::Match {
            source: &source.tracks[0],
            destination: &destination.tracks[0],
        }];
        assert_eq!(match_tracks(&source, &destination).unwrap(), expected);
    }

    #[test]
    fn test_match_tracks_same_title_track() {
        let source = Album {
            catalog_id: "10".to_owned(),
            name: "Album 1".to_owned(),
            artist_name: "Artist".to_owned(),
            release_date: "2020-01-01".to_owned(),
            tracks: vec![TrackWithLibrary {
                catalog_id: "1".to_owned(),
                name: "Song 1".to_owned(),
                artist_name: "Artist".to_owned(),
                is_explicit: false,
                isrc: "ISRC1".to_owned(),
                release_date: "2020-01-01".to_owned(),
                library_id: None,
            }],
        };
        let destination = Album {
            catalog_id: "11".to_owned(),
            name: "Album 1".to_owned(),
            artist_name: "Artist".to_owned(),
            release_date: "2020-01-01".to_owned(),
            tracks: vec![TrackNoLibrary {
                catalog_id: "2".to_owned(),
                name: "Song 1".to_owned(),
                artist_name: "Artist 2".to_owned(),
                is_explicit: false,
                isrc: "ISRC2".to_owned(),
                release_date: "2020-01-02".to_owned(),
            }],
        };
        let expected = vec![TrackMatchResult::NoMatch {
            source: &source.tracks[0],
        }];
        assert_eq!(match_tracks(&source, &destination).unwrap(), expected);
    }

    #[test]
    fn test_match_tracks_clean() {
        let source = Album {
            catalog_id: "10".to_owned(),
            name: "Album 1".to_owned(),
            artist_name: "Artist".to_owned(),
            release_date: "2020-01-01".to_owned(),
            tracks: vec![
                TrackWithLibrary {
                    catalog_id: "11".to_owned(),
                    name: "Song 1".to_owned(),
                    artist_name: "Artist".to_owned(),
                    is_explicit: false,
                    isrc: "ISRC11".to_owned(),
                    release_date: "2020-01-01".to_owned(),
                    library_id: None,
                },
                TrackWithLibrary {
                    catalog_id: "21".to_owned(),
                    name: "Song 2".to_owned(),
                    artist_name: "Artist".to_owned(),
                    is_explicit: false,
                    isrc: "ISRC21".to_owned(),
                    release_date: "2020-01-01".to_owned(),
                    library_id: Some("i.2".to_owned()),
                },
            ],
        };
        let destination = Album {
            catalog_id: "11".to_owned(),
            name: "Album 1".to_owned(),
            artist_name: "Artist".to_owned(),
            release_date: "2020-01-01".to_owned(),
            tracks: vec![
                TrackNoLibrary {
                    catalog_id: "12".to_owned(),
                    name: "Song 1".to_owned(),
                    artist_name: "Artist".to_owned(),
                    is_explicit: true,
                    isrc: "ISRC12".to_owned(),
                    release_date: "2020-01-01".to_owned(),
                },
                TrackNoLibrary {
                    catalog_id: "22".to_owned(),
                    name: "Song 2".to_owned(),
                    artist_name: "Artist".to_owned(),
                    is_explicit: true,
                    isrc: "ISRC22".to_owned(),
                    release_date: "2020-01-01".to_owned(),
                },
            ],
        };
        let expected = vec![
            TrackMatchResult::Match {
                source: &source.tracks[0],
                destination: &destination.tracks[0],
            },
            TrackMatchResult::Match {
                source: &source.tracks[1],
                destination: &destination.tracks[1],
            },
        ];
        assert_eq!(match_tracks(&source, &destination).unwrap(), expected);
    }

    #[test]
    fn test_match_tracks_duplicate_isrc_source() {
        let source = Album {
            catalog_id: "10".to_owned(),
            name: "Album 1".to_owned(),
            artist_name: "Artist".to_owned(),
            release_date: "2020-01-01".to_owned(),
            tracks: vec![
                TrackWithLibrary {
                    catalog_id: "1".to_owned(),
                    name: "Song 1".to_owned(),
                    artist_name: "Artist".to_owned(),
                    is_explicit: false,
                    isrc: "ISRC1".to_owned(),
                    release_date: "2020-01-01".to_owned(),
                    library_id: None,
                },
                TrackWithLibrary {
                    catalog_id: "2".to_owned(),
                    name: "Song 2".to_owned(),
                    artist_name: "Artist".to_owned(),
                    is_explicit: false,
                    isrc: "ISRC1".to_owned(),
                    release_date: "2020-01-01".to_owned(),
                    library_id: None,
                },
            ],
        };
        let destination = Album {
            catalog_id: "11".to_owned(),
            name: "Album 1".to_owned(),
            artist_name: "Artist".to_owned(),
            release_date: "2020-01-01".to_owned(),
            tracks: vec![TrackNoLibrary {
                catalog_id: "3".to_owned(),
                name: "Song 1".to_owned(),
                artist_name: "Artist".to_owned(),
                is_explicit: false,
                isrc: "ISRC1".to_owned(),
                release_date: "2020-01-02".to_owned(),
            }],
        };
        assert!(match_tracks(&source, &destination).is_err());
    }

    #[test]
    fn test_match_tracks_duplicate_isrc_destination() {
        let source = Album {
            catalog_id: "10".to_owned(),
            name: "Album 1".to_owned(),
            artist_name: "Artist".to_owned(),
            release_date: "2020-01-01".to_owned(),
            tracks: vec![TrackWithLibrary {
                catalog_id: "1".to_owned(),
                name: "Song 1".to_owned(),
                artist_name: "Artist".to_owned(),
                is_explicit: false,
                isrc: "ISRC1".to_owned(),
                release_date: "2020-01-01".to_owned(),
                library_id: None,
            }],
        };
        let destination = Album {
            catalog_id: "11".to_owned(),
            name: "Album 1".to_owned(),
            artist_name: "Artist".to_owned(),
            release_date: "2020-01-01".to_owned(),
            tracks: vec![
                TrackNoLibrary {
                    catalog_id: "2".to_owned(),
                    name: "Song 1".to_owned(),
                    artist_name: "Artist".to_owned(),
                    is_explicit: false,
                    isrc: "ISRC1".to_owned(),
                    release_date: "2020-01-02".to_owned(),
                },
                TrackNoLibrary {
                    catalog_id: "3".to_owned(),
                    name: "Song 1".to_owned(),
                    artist_name: "Artist".to_owned(),
                    is_explicit: false,
                    isrc: "ISRC1".to_owned(),
                    release_date: "2020-01-02".to_owned(),
                },
            ],
        };
        assert!(match_tracks(&source, &destination).is_err());
    }

    #[test]
    fn test_match_tracks_duplicate_catalog_id_source() {
        let source = Album {
            catalog_id: "10".to_owned(),
            name: "Album 1".to_owned(),
            artist_name: "Artist".to_owned(),
            release_date: "2020-01-01".to_owned(),
            tracks: vec![
                TrackWithLibrary {
                    catalog_id: "1".to_owned(),
                    name: "Song 1".to_owned(),
                    artist_name: "Artist".to_owned(),
                    is_explicit: false,
                    isrc: "ISRC1".to_owned(),
                    release_date: "2020-01-01".to_owned(),
                    library_id: None,
                },
                TrackWithLibrary {
                    catalog_id: "1".to_owned(),
                    name: "Song 2".to_owned(),
                    artist_name: "Artist".to_owned(),
                    is_explicit: false,
                    isrc: "ISRC2".to_owned(),
                    release_date: "2020-01-01".to_owned(),
                    library_id: None,
                },
            ],
        };
        let destination = Album {
            catalog_id: "11".to_owned(),
            name: "Album 1".to_owned(),
            artist_name: "Artist".to_owned(),
            release_date: "2020-01-01".to_owned(),
            tracks: vec![TrackNoLibrary {
                catalog_id: "2".to_owned(),
                name: "Song 1".to_owned(),
                artist_name: "Artist".to_owned(),
                is_explicit: false,
                isrc: "ISRC1".to_owned(),
                release_date: "2020-01-02".to_owned(),
            }],
        };
        assert!(match_tracks(&source, &destination).is_err());
    }

    #[test]
    fn test_match_tracks_duplicate_catalog_id_destination() {
        let source = Album {
            catalog_id: "10".to_owned(),
            name: "Album 1".to_owned(),
            artist_name: "Artist".to_owned(),
            release_date: "2020-01-01".to_owned(),
            tracks: vec![TrackWithLibrary {
                catalog_id: "1".to_owned(),
                name: "Song 1".to_owned(),
                artist_name: "Artist".to_owned(),
                is_explicit: false,
                isrc: "ISRC1".to_owned(),
                release_date: "2020-01-01".to_owned(),
                library_id: None,
            }],
        };
        let destination = Album {
            catalog_id: "11".to_owned(),
            name: "Album 1".to_owned(),
            artist_name: "Artist".to_owned(),
            release_date: "2020-01-01".to_owned(),
            tracks: vec![
                TrackNoLibrary {
                    catalog_id: "2".to_owned(),
                    name: "Song 1".to_owned(),
                    artist_name: "Artist".to_owned(),
                    is_explicit: false,
                    isrc: "ISRC1".to_owned(),
                    release_date: "2020-01-02".to_owned(),
                },
                TrackNoLibrary {
                    catalog_id: "2".to_owned(),
                    name: "Song 1".to_owned(),
                    artist_name: "Artist".to_owned(),
                    is_explicit: false,
                    isrc: "ISRC1".to_owned(),
                    release_date: "2020-01-02".to_owned(),
                },
            ],
        };
        assert!(match_tracks(&source, &destination).is_err());
    }

    #[test]
    fn test_match_tracks_multiple_same_title_artist_track() {
        let source = Album {
            catalog_id: "10".to_owned(),
            name: "Album 1".to_owned(),
            artist_name: "Artist".to_owned(),
            release_date: "2020-01-01".to_owned(),
            tracks: vec![TrackWithLibrary {
                catalog_id: "1".to_owned(),
                name: "Song 1".to_owned(),
                artist_name: "Artist".to_owned(),
                is_explicit: false,
                isrc: "ISRC1".to_owned(),
                release_date: "2020-01-01".to_owned(),
                library_id: None,
            }],
        };
        let destination = Album {
            catalog_id: "11".to_owned(),
            name: "Album 1".to_owned(),
            artist_name: "Artist".to_owned(),
            release_date: "2020-01-01".to_owned(),
            tracks: vec![
                TrackNoLibrary {
                    catalog_id: "2".to_owned(),
                    name: "Song 1".to_owned(),
                    artist_name: "Artist".to_owned(),
                    is_explicit: false,
                    isrc: "ISRC2".to_owned(),
                    release_date: "2020-01-02".to_owned(),
                },
                TrackNoLibrary {
                    catalog_id: "3".to_owned(),
                    name: "Song 1".to_owned(),
                    artist_name: "Artist".to_owned(),
                    is_explicit: false,
                    isrc: "ISRC3".to_owned(),
                    release_date: "2020-01-02".to_owned(),
                },
            ],
        };
        assert!(match_tracks(&source, &destination).is_err());
    }
}
