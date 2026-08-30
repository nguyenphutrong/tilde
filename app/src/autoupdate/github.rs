use anyhow::{Context as _, Result};
use channel_versions::VersionInfo;
use semver::Version;
use serde::Deserialize;

const TILDE_RELEASES_API_URL: &str =
    "https://api.github.com/repos/nguyenphutrong/tilde/releases?per_page=20";

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    draft: bool,
    assets: Vec<GithubReleaseAsset>,
}

#[derive(Debug, Deserialize)]
struct GithubReleaseAsset {
    name: String,
}

fn expected_asset_names(tag: &str) -> [String; 2] {
    [
        format!("Tilde-{tag}-macos-arm64.dmg"),
        format!("Tilde-{tag}-macos-arm64.sha256"),
    ]
}

fn latest_installable_version(releases: Vec<GithubRelease>) -> Result<VersionInfo> {
    releases
        .into_iter()
        .filter(|release| {
            !release.draft
                && expected_asset_names(&release.tag_name)
                    .iter()
                    .all(|expected| release.assets.iter().any(|asset| asset.name == *expected))
        })
        .filter_map(|release| {
            Version::parse(release.tag_name.trim_start_matches('v'))
                .ok()
                .map(|version| (version, release.tag_name))
        })
        .max_by(|(left, _), (right, _)| left.cmp(right))
        .map(|(_, tag)| VersionInfo::new(tag))
        .context("No installable Tilde release found on GitHub")
}

pub(super) async fn fetch_latest_tilde_version(
    client: &http_client::Client,
) -> Result<VersionInfo> {
    let response = client
        .get(TILDE_RELEASES_API_URL)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "tilde-terminal")
        .send()
        .await
        .context("Failed to fetch Tilde releases from GitHub")?
        .error_for_status()?;
    let releases = response
        .json()
        .await
        .context("Failed to parse Tilde releases from GitHub")?;
    latest_installable_version(releases)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(tag: &str, draft: bool, complete: bool) -> GithubRelease {
        let assets = if complete {
            expected_asset_names(tag)
                .into_iter()
                .map(|name| GithubReleaseAsset { name })
                .collect()
        } else {
            vec![]
        };
        GithubRelease {
            tag_name: tag.to_owned(),
            draft,
            assets,
        }
    }

    #[test]
    fn selects_highest_installable_release_including_prereleases() {
        let version = latest_installable_version(vec![
            release("v0.1.2", false, true),
            release("v0.2.0-beta.1", false, true),
            release("v0.1.9", false, true),
        ])
        .unwrap();

        assert_eq!(version.version, "v0.2.0-beta.1");
    }

    #[test]
    fn ignores_drafts_and_incomplete_releases() {
        let version = latest_installable_version(vec![
            release("v0.3.0", true, true),
            release("v0.2.0", false, false),
            release("v0.1.2", false, true),
        ])
        .unwrap();

        assert_eq!(version.version, "v0.1.2");
    }
}
