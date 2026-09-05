use serde::{Deserialize, Serialize};
use worker::*;

#[derive(Deserialize)]
struct NewEvent {
    id: String,
    name: String,
    organizer_capability: String,
    response_capability: String,
}

#[derive(Deserialize)]
struct NewAnswer {
    event_id: String,
    respondent: String,
    availability: String,
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
            if input.id.is_empty() || input.id.len() > 64 || input.name.trim().is_empty() || input.organizer_capability.len() < 16 || input.response_capability.len() < 16 {
                return Response::error("invalid event", 400);
            }
            let db = ctx.env.d1("DB")?;
            db.batch(vec![
                db.prepare("INSERT INTO events(id, name, organizer_capability, response_capability) VALUES(?1, ?2, ?3, ?4)")
                    .bind(&[input.id.clone().into(), input.name.clone().into(), input.organizer_capability.clone().into(), input.response_capability.clone().into()])?,
            ])
            .await?;
            Response::from_json(&Event {
                id: input.id,
                name: input.name,
            })
        })
        .post_async("/api/answers", |mut request, ctx| async move {
            let input: NewAnswer = request.json().await?;
            let capability = request.headers().get("x-response-capability")?.ok_or_else(|| Error::RustError("missing capability".into()))?;
            if input.event_id.is_empty() || input.respondent.trim().is_empty() || input.availability.trim().is_empty() { return Response::error("invalid answer", 400); }
            let db = ctx.env.d1("DB")?;
            let event = db.prepare("SELECT response_capability FROM events WHERE id = ?1").bind(&[input.event_id.clone().into()])?.first::<serde_json::Value>(None).await?;
            if event.as_ref().and_then(|v| v.get("response_capability")).and_then(|v| v.as_str()) != Some(capability.as_str()) { return Response::error("forbidden", 403); }
            db.prepare("INSERT INTO answers(event_id, respondent, availability) VALUES(?1, ?2, ?3)").bind(&[input.event_id.clone().into(), input.respondent.clone().into(), input.availability.clone().into()])?.run().await?;
            Response::from_json(&serde_json::json!({"event_id": input.event_id, "respondent": input.respondent, "availability": input.availability}))
        })
        .run(request, env)
        .await
}
