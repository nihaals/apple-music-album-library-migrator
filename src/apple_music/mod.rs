mod api_types;
mod client;
pub mod custom_types;

pub use client::Client;

pub fn validate_catalog_id(id: &str) -> bool {
    id.chars().all(|c| c.is_ascii_digit())
}

pub fn validate_library_album_id(id: &str) -> bool {
    id.starts_with("l.") && id[2..].chars().all(|c| c.is_ascii_alphanumeric())
}

/// Run some basic checks to validate the developer token
pub fn validate_developer_token(token: &str) -> bool {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return false;
    }

    // TODO: Check alg
    // TODO: Check kid exists
    // TODO: Check iss exists
    // TODO: Check iat exists and is valid
    // TODO: Check exp exists and is valid

    true
}

pub fn validate_storefront(storefront: &str) -> bool {
    storefront.len() == 2 && storefront.chars().all(|c| c.is_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_developer_token_apple_music_web() {
        let token = "eyJhbGciOiJFUzI1NiIsInR5cCI6IkpXVCIsImtpZCI6IldlYlBsYXlLaWQifQ.eyJpc3MiOiJBTVBXZWJQbGF5IiwiaWF0IjoxNzcwODcxMjQ5LCJleHAiOjE3NzgxMjg4NDksInJvb3RfaHR0cHNfb3JpZ2luIjpbImFwcGxlLmNvbSJdfQ.7Zj7Zb4kkn7PUlTpFZxF5Fb1zv_WBRROmZuM3IBdvrkhkYzUs3eXyyiuhW_vbOXVeibrQVRjTnvU-Zr4v4w_Bg";
        assert!(validate_developer_token(token));
    }
}
