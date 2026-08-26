use super::{OzConfig, WarpServerConfig};

#[test]
fn offline_configs_only_target_loopback() {
    let server = WarpServerConfig::offline();
    let oz = OzConfig::offline();

    assert_eq!(server.server_root_url, "http://127.0.0.1:0");
    assert_eq!(server.rtc_server_url, "ws://127.0.0.1:0");
    assert!(server.session_sharing_server_url.is_none());
    assert!(server.firebase_auth_api_key.is_empty());
    assert_eq!(oz.oz_root_url, "http://127.0.0.1:0");
}
