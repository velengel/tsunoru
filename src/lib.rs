//! TSUNORU's shared Dioxus application, domain model, and server boundary.

use dioxus::prelude::*;
use ui::{
    ContinueHistoryEvent, Create, History, HistoryEvent, Login, OrganizerSummary, Register,
    SharedEvent,
};

#[cfg(feature = "server")]
pub mod auth;
#[cfg(feature = "server")]
pub mod calendar;
pub mod domain;
pub mod server;
#[cfg(feature = "server")]
pub mod storage;
pub mod ui;

const MAIN_CSS: Asset = asset!("/assets/main.css");
const FAVICON: Asset = asset!("/assets/favicon.png");

/// Browser routes kept in the same Rust type as their rendered components.
#[derive(Clone, Debug, PartialEq, Routable)]
pub enum Route {
    #[route("/")]
    Create {},
    #[route("/register")]
    Register {},
    #[route("/login")]
    Login {},
    #[route("/history")]
    History {},
    #[route("/history/events/:public_id/continue")]
    ContinueHistoryEvent { public_id: String },
    #[route("/history/events/:public_id")]
    HistoryEvent { public_id: String },
    #[route("/events/:public_id/summary")]
    OrganizerSummary { public_id: String },
    #[route("/events/:public_id")]
    SharedEvent { public_id: String },
}

/// Fullstack application entry shared by the web and server builds.
#[component]
pub fn App() -> Element {
    rsx! {
        document::Title { "TSUNORU" }
        document::Meta {
            name: "description",
            content: "友人や仲間と集まる日を決める日程調整アプリ、TSUNORU。",
        }
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        Router::<Route> {}
    }
}
