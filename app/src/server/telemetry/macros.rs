/// Records a telemetry event in the local queue from a background executor.
#[macro_export]
macro_rules! send_telemetry_on_executor {
    ($auth_state: expr_2021, $event:expr_2021, $executor:expr_2021) => {
        #[allow(unused_imports)]
        use warp_core::telemetry::TelemetryEvent as _;
        let event = $event;
        if event.enablement_state().is_enabled() {
            let user_id = $auth_state.user_id().map(|uid| uid.as_string());
            let anonymous_id = $auth_state.anonymous_id();
            warpui::record_telemetry_on_executor!(
                user_id,
                anonymous_id,
                event.name().into(),
                event.payload(),
                event.contains_ugc(),
                $executor
            );
        }
    };
}
