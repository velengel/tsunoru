use tsunoru::App;

fn main() {
    #[cfg(feature = "server")]
    dioxus::serve(|| async move {
        use dioxus::server::axum::{middleware, routing::get};

        Ok(dioxus::server::router(App)
            .route(
                "/api/events/{public_id}/calendar.ics",
                get(tsunoru::server::download_public_calendar),
            )
            .layer(middleware::from_fn(
                tsunoru::server::require_same_origin_api,
            )))
    });

    #[cfg(not(feature = "server"))]
    dioxus::launch(App);
}
