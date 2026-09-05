use serde::{Deserialize, Serialize};
use worker::*;

#[derive(Deserialize)]
struct NewEvent {
    id: String,
    name: String,
}

#[derive(Serialize)]
struct Event {
    id: String,
    name: String,
}

#[event(fetch)]
pub async fn fetch(request: Request, env: Env, _ctx: Context) -> Result<Response> {
    Router::with_data(env.clone())
        .get_async("/health", |_request, _ctx| async move {
            Response::from_json(&serde_json::json!({ "status": "ok", "runtime": "rust-worker" }))
        })
        .post_async("/api/events", |mut request, ctx| async move {
            let input: NewEvent = request.json().await?;
            if input.id.is_empty() || input.id.len() > 64 || input.name.trim().is_empty() {
                return Response::error("invalid event", 400);
            }
            let db = ctx.env.d1("DB")?;
            db.batch(vec![
                db.prepare("INSERT INTO events(id, name) VALUES(?1, ?2)")
                    .bind(&[input.id.clone().into(), input.name.clone().into()])?,
            ])
            .await?;
            Response::from_json(&Event {
                id: input.id,
                name: input.name,
            })
        })
        .run(request, env)
        .await
}
