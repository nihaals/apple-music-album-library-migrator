use anyhow::Result;

use crate::apple_music::api_types;

pub struct Client {
    client: reqwest::Client,
    user_token: String,
    storefront: String,
}

impl Client {
    pub fn new(
        developer_token: &str,
        origin_header: Option<String>,
        user_token: String,
        storefront: String,
    ) -> Result<Self> {
        let headers = {
            let mut headers = reqwest::header::HeaderMap::new();
            headers.insert(
                "Authorization",
                format!("Bearer {}", developer_token).try_into()?,
            );
            if let Some(origin) = origin_header {
                headers.insert("Origin", origin.try_into()?);
            }
            headers
        };
        let client = reqwest::Client::builder()
            .default_headers(headers)
            .danger_accept_invalid_certs(true) // TODO: Remove
            .build()?;
        Ok(Self {
            client,
            user_token,
            storefront,
        })
    }

    pub async fn get_catalog_album(
        &self,
        catalog_id: &str,
    ) -> Result<api_types::catalog_album::Root> {
        Ok(self
            .client
            .get(format!(
                "https://amp-api.music.apple.com/v1/catalog/{}/albums/{catalog_id}",
                self.storefront,
            ))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?)
    }

    pub async fn get_library_album(
        &self,
        library_id: &str,
    ) -> Result<api_types::library_album::Root> {
        Ok(self
            .client
            .get(format!(
                "https://amp-api.music.apple.com/v1/me/library/albums/{library_id}?include=catalog",
            ))
            .header("Media-User-Token", &self.user_token)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?)
    }

    pub async fn add_songs_to_library(&self, catalog_ids: &[&str]) -> Result<()> {
        let ids = catalog_ids.join(",");
        self.client
            .post(format!(
                "https://amp-api.music.apple.com/v1/me/library?ids[songs]={ids}",
            ))
            .header("Media-User-Token", &self.user_token)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    pub async fn remove_album_from_library(&self, library_id: &str) -> Result<()> {
        self.client
            .delete(format!(
                "https://amp-api.music.apple.com/v1/me/library/albums/{library_id}",
            ))
            .header("Media-User-Token", &self.user_token)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }
}
