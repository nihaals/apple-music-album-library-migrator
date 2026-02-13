use anyhow::{Result, ensure};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Root {
    pub(in crate::apple_music) data: Vec<LibraryAlbum>,
}

#[derive(Deserialize)]
pub struct LibraryAlbum {
    pub(in crate::apple_music) id: String,
    pub(in crate::apple_music) relationships: LibraryAlbumRelationshipsWithTracksCatalog,
}

#[derive(Deserialize)]
pub struct LibraryAlbumRelationshipsWithTracksCatalog {
    pub(in crate::apple_music) catalog: LibraryAlbumRelationshipsCatalog,
    pub(in crate::apple_music) tracks: LibraryAlbumRelationshipsTracks,
}

#[derive(Deserialize)]
pub struct LibraryAlbumRelationshipsCatalog {
    pub(in crate::apple_music) data: Vec<LibraryAlbumCatalog>,
}

#[derive(Deserialize)]
pub struct LibraryAlbumCatalog {
    pub(in crate::apple_music) id: String,
}

#[derive(Deserialize)]
pub struct LibraryAlbumRelationshipsTracks {
    pub(in crate::apple_music) data: Vec<LibrarySong>,
}

#[derive(Deserialize)]
pub struct LibrarySong {
    pub(in crate::apple_music) attributes: LibrarySongAttributes,
    pub(in crate::apple_music) id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibrarySongAttributes {
    pub(in crate::apple_music) play_params: LibrarySongPlayParams,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibrarySongPlayParams {
    pub(in crate::apple_music) catalog_id: String,
}

impl Root {
    pub fn catalog_id(&self) -> Result<&str> {
        ensure!(self.data.len() == 1);
        let album = &self.data[0];
        ensure!(album.relationships.catalog.data.len() == 1);
        Ok(&album.relationships.catalog.data[0].id)
    }

    pub fn library_id(&self) -> Result<&str> {
        ensure!(self.data.len() == 1);
        let album = &self.data[0];
        Ok(&album.id)
    }
}
