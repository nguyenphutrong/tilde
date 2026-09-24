use anyhow::{Context as _, Result};
use channel_versions::{Changelog, MarkdownSection};
use chrono::{DateTime, FixedOffset};
use serde::Deserialize;

use crate::channel::ChannelState;

#[derive(Deserialize)]
struct ReleaseNotes {
    published_at: DateTime<FixedOffset>,
    body: Option<String>,
}

pub async fn get_current_changelog(client: &http_client::Client) -> Result<Option<Changelog>> {
    let Some(version) = ChannelState::app_version() else {
        return Ok(None);
    };
    // Local builds have no published release notes.
    if semver::Version::parse(version.trim_start_matches('v')).is_err() {
        return Ok(None);
    }
    let url = format!("https://api.github.com/repos/nguyenphutrong/tilde/releases/tags/{version}");
    let response = client
        .get(&url)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "tilde-terminal")
        .send()
        .await?
        .error_for_status()?;
    let notes: ReleaseNotes = response
        .json()
        .await
        .context("Failed to parse Tilde release notes")?;
    Ok(Some(Changelog {
        date: notes.published_at,
        sections: Vec::new(),
        markdown_sections: vec![MarkdownSection {
            title: "Release notes".to_owned(),
            markdown: notes.body.unwrap_or_default(),
        }],
        image_url: None,
        oz_updates: Vec::new(),
        tui_updates: Vec::new(),
    }))
}
