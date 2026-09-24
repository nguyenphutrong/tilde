//! Repository detection for local terminal sessions.

use std::future::Future;

#[cfg(not(target_family = "wasm"))]
use futures::future::Either;
use futures::future::ready;
#[cfg(not(target_family = "wasm"))]
use repo_metadata::repositories::DetectedRepositories;
use repo_metadata::repositories::RepoDetectionSource;
use warp_util::local_or_remote_path::LocalOrRemotePath;
use warpui::AppContext;
#[cfg(not(target_family = "wasm"))]
use warpui::SingletonEntity;

/// Describes whether the active session is local or remote.
pub enum RepoDetectionSessionType {
    /// A local terminal session — repo detection runs on the local filesystem.
    Local,
    /// A remote SSH session whose paths must not be resolved locally.
    Remote,
}

/// Detects the git repository root of a local session's working directory.
#[cfg(not(target_family = "wasm"))]
pub fn detect_possible_git_repo(
    session_type: RepoDetectionSessionType,
    active_directory: &str,
    source: RepoDetectionSource,
    ctx: &mut AppContext,
) -> impl Future<Output = Option<LocalOrRemotePath>> + use<> {
    match session_type {
        RepoDetectionSessionType::Local => {
            let detection = DetectedRepositories::handle(ctx).update(ctx, |repos, ctx| {
                repos.detect_possible_local_git_repo(active_directory, source, ctx)
            });
            Either::Left(async move { detection.await.map(LocalOrRemotePath::Local) })
        }
        RepoDetectionSessionType::Remote => Either::Right(ready(None)),
    }
}

/// Repository detection is not available in WASM builds because
/// `DetectedRepositories` is not registered there.
#[cfg(target_family = "wasm")]
pub fn detect_possible_git_repo(
    _session_type: RepoDetectionSessionType,
    _active_directory: &str,
    _source: RepoDetectionSource,
    _ctx: &mut AppContext,
) -> impl Future<Output = Option<LocalOrRemotePath>> + use<> {
    ready(None)
}

#[cfg(all(test, feature = "local_fs", not(target_family = "wasm")))]
#[path = "repo_detection_tests.rs"]
mod tests;
