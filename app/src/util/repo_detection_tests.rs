use command::blocking::Command;
use repo_metadata::DirectoryWatcher;
use tempfile::TempDir;
use warpui::App;

use super::{
    DetectedRepositories, LocalOrRemotePath, RepoDetectionSessionType, RepoDetectionSource,
    detect_possible_git_repo,
};

#[test]
fn only_local_sessions_resolve_a_local_repository() {
    let directory = TempDir::new().unwrap();
    let output = Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(directory.path())
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let root = dunce::canonicalize(directory.path()).unwrap();
    let nested = root.join("nested");
    std::fs::create_dir(&nested).unwrap();

    App::test((), |mut app| async move {
        let remote = app
            .update(|ctx| {
                detect_possible_git_repo(
                    RepoDetectionSessionType::Remote,
                    nested.to_str().unwrap(),
                    RepoDetectionSource::TerminalNavigation,
                    ctx,
                )
            })
            .await;
        assert_eq!(remote, None);

        app.add_singleton_model(DirectoryWatcher::new);
        app.add_singleton_model(|_| DetectedRepositories::default());
        let local = app
            .update(|ctx| {
                detect_possible_git_repo(
                    RepoDetectionSessionType::Local,
                    nested.to_str().unwrap(),
                    RepoDetectionSource::TerminalNavigation,
                    ctx,
                )
            })
            .await;
        assert_eq!(local, Some(LocalOrRemotePath::Local(root)));
    });
}
