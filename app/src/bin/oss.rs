// On Windows, we don't want to display a console window when the application is running in release
// builds. See https://doc.rust-lang.org/reference/runtime.html#the-windows_subsystem-attribute.
#![cfg_attr(feature = "release_bundle", windows_subsystem = "windows")]

use anyhow::Result;
use warp_core::AppId;
use warp_core::channel::{
    AutoupdateConfig, Channel, ChannelConfig, ChannelState, OzConfig, WarpServerConfig,
};

// Simple wrapper around warp::run() for Tilde builds.
fn main() -> Result<()> {
    let mut state = ChannelState::new(
        Channel::Oss,
        ChannelConfig {
            app_id: AppId::new("bytrong", "app", "tilde"),
            logfile_name: "tilde.log".into(),
            server_config: WarpServerConfig::offline(),
            oz_config: OzConfig::offline(),
            telemetry_config: None,
            crash_reporting_config: None,
            autoupdate_config: Some(AutoupdateConfig {
                releases_base_url: "https://github.com/nguyenphutrong/tilde/releases".into(),
                show_autoupdate_menu_items: cfg!(all(target_os = "macos", target_arch = "aarch64")),
            }),
            mcp_static_config: None,
        },
    );
    if cfg!(debug_assertions) {
        state = state.with_additional_features(warp_core::features::DEBUG_FLAGS);
    }
    ChannelState::set(state);

    warp::run()
}

// If we're not using an external plist, embed the following as the Info.plist.
#[cfg(all(not(feature = "extern_plist"), target_os = "macos"))]
embed_plist::embed_info_plist_bytes!(r#"
    <?xml version="1.0" encoding="UTF-8"?>
    <!DOCTYPE plist PUBLIC "-//Apple Computer//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
    <plist version="1.0">
    <dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>English</string>
    <key>CFBundleDisplayName</key>
    <string>Tilde</string>
    <key>CFBundleExecutable</key>
    <string>tilde</string>
    <key>CFBundleIdentifier</key>
    <string>bytrong.app.tilde</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>Tilde</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>0.1.3</string>
    <key>LSApplicationCategoryType</key>
    <string>public.app-category.developer-tools</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>UIDesignRequiresCompatibility</key>
    <true/>
    <key>CFBundleURLTypes</key>
    <array><dict><key>CFBundleURLName</key><string>Tilde</string><key>CFBundleURLSchemes</key><array><string>tilde</string></array></dict></array>
    <key>NSHumanReadableCopyright</key>
    <string>© 2026 Trong Nguyen</string>
    </dict>
    </plist>
"#.as_bytes());
