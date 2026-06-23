// Local development binary for Oz agent runs.
// Uses Channel::Local which allows --server-root-url overrides, enabling
// the oz binary to authenticate against a local warp-server instance.
//
// Build with: cargo build --bin oz-local-dev
// Use with:   oz-local-dev agent run --task-id <id> --sandboxed
//             --server-root-url http://localhost:8080

use anyhow::Result;
use warp_core::channel::{
    Channel, ChannelConfig, ChannelState, OzConfig, WarpServerConfig,
};
use warp_core::AppId;
use warp_core::features::FeatureFlag;

fn main() -> Result<()> {
    let mut state = ChannelState::new(
        Channel::Local,
        ChannelConfig {
            app_id: AppId::new("dev", "warp", "OzLocalDev"),
            logfile_name: "oz-local-dev.log".into(),
            server_config: WarpServerConfig {
                // Default to the local main server. Can be overridden via
                // --server-root-url or WARP_SERVER_ROOT_URL.
                server_root_url: "http://localhost:8080".into(),
                rtc_server_url: "ws://localhost:8080/graphql/v2".into(),
                // No session sharing server — tasks run in headless/sandboxed mode
                // without interactive session sharing.
                session_sharing_server_url: None,
                // Firebase API key — used for token refresh; in sandboxed/API-key
                // mode this path is not exercised so any non-empty string is fine.
                firebase_auth_api_key: "local-dev-no-firebase".into(),
                iap_config: None,
            },
            oz_config: OzConfig {
                oz_root_url: "http://localhost:8082".into(),
                // Use the staging workload audience so Namespace can issue a valid
                // JWT for this binary (nsc only allows known audiences). The local
                // server accepts it because IGNORE_WORKLOAD_TOKEN_VERIFICATION=true.
                workload_audience_url: Some("https://staging.warp.dev".into()),
            },
            telemetry_config: None,
            crash_reporting_config: None,
            autoupdate_config: None,
            mcp_static_config: None,
        },
    );

    // Enable debug and dogfood flags for local development.
    state = state.with_additional_features(warp_core::features::DEBUG_FLAGS);
    state = state.with_additional_features(warp_core::features::DOGFOOD_FLAGS);
    ChannelState::set(state);

    // Disable session sharing — there is no session-sharing server running
    // in this local dev environment.
    FeatureFlag::AgentSharedSessions.set_enabled(false);
    FeatureFlag::CreatingSharedSessions.set_enabled(false);

    warp::run()
}
