//! Sinalização e drenagem graciosa do servidor.

pub async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let mut term = signal(SignalKind::terminate()).expect("falha ao registrar SIGTERM");
        tokio::select! { _ = tokio::signal::ctrl_c() => {}, _ = term.recv() => {} }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
    crate::operability::runtime_state().stop_accepting();
    let drained = crate::operability::runtime_state()
        .drain(std::time::Duration::from_secs(
            crate::config::settings().shutdown_timeout_secs,
        ))
        .await;
    if !drained {
        crate::security::log_event(
            "graceful_shutdown_timeout",
            serde_json::json!({ "in_flight": crate::operability::runtime_state().in_flight() }),
        );
    } else {
        crate::security::log_event("graceful_shutdown_completed", serde_json::json!({}));
    }
}
