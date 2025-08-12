use anyhow::Result;

use hayride_host_traits::core::version::{errors::ErrorCode, VersionInner};

#[derive(Clone, Default)]
pub struct VersionBackend {}

impl VersionInner for VersionBackend {
    fn latest(&self) -> Result<String, ErrorCode> {
        // Get the latest version from Hayride releases
        let client = reqwest::blocking::Client::new();
        let response = match client
            .get("https://api.github.com/repos/hayride-dev/releases/releases/latest")
            .header(reqwest::header::USER_AGENT, "Hayride")
            .send()
        {
            Ok(resp) => resp,
            Err(_) => {
                return Err(ErrorCode::GetVersionFailed);
            }
        };

        // Parse the tag
        let json: serde_json::Value = response.json().map_err(|_| ErrorCode::GetVersionFailed)?;
        let tag_name = json
            .get("tag_name")
            .and_then(|v| v.as_str())
            .ok_or(ErrorCode::GetVersionFailed)?;
        Ok(tag_name.into())
    }
}
