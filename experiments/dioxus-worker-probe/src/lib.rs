use dioxus::prelude::*;

#[post("/api/probe")]
pub async fn probe() -> Result<String, ServerFnError> {
    Ok("worker-probe".into())
}
