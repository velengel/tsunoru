//! Same-origin REST calls. The trial code only exists during its login request.
use super::{CreateRequest, Event, ResponseRecord, ResponseView};
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApiError {
    pub status: u16,
    pub code: String,
}

impl ApiError {
    fn network() -> Self {
        Self {
            status: 0,
            code: "network_error".to_owned(),
        }
    }
    fn invalid() -> Self {
        Self {
            status: 0,
            code: "invalid_response".to_owned(),
        }
    }
    pub fn needs_access(&self) -> bool {
        self.status == 401
    }
    pub fn message(&self) -> &'static str {
        match self.status {
            401 => "利用時間が終了しました。試用コードを入力すると、保存した内容で再開できます。",
            403 => "この操作の権限を確認できませんでした。",
            404 => "イベントが見つかりません。共有URLを確認してください。",
            409 => {
                "この送信内容と、すでに保存された内容が一致しません。別の回答を作らず、主催者へ確認してください。"
            }
            400 | 413 | 415 | 422 => {
                "入力内容を保存できませんでした。日時やタイムゾーンを確認してください。"
            }
            _ => {
                "接続できませんでした。送信内容は残っています。少し待ってから、もう一度お試しください。"
            }
        }
    }
}

pub async fn session() -> Result<(), ApiError> {
    request("GET", "/api/staging/session", None, None)
        .await
        .map(|_| ())
}

pub async fn organizer_session_status() -> Result<(), ApiError> {
    request("GET", "/api/organizer/session", None, None)
        .await
        .map(|_| ())
}

pub async fn organizer_session(id_token: String, nonce: String) -> Result<(), ApiError> {
    request(
        "POST",
        "/api/organizer/session",
        Some(json!({"id_token": id_token, "nonce": nonce})),
        None,
    )
    .await
    .map(|_| ())
}
pub async fn login(code: String) -> Result<(), ApiError> {
    request(
        "POST",
        "/api/staging/session",
        Some(json!({"access_code":code})),
        None,
    )
    .await
    .map(|_| ())
}
pub async fn logout() -> Result<(), ApiError> {
    request("DELETE", "/api/staging/session", None, None)
        .await
        .map(|_| ())
}

pub async fn organizer_logout() -> Result<(), ApiError> {
    request("DELETE", "/api/organizer/session", None, None)
        .await
        .map(|_| ())
}
pub async fn create(input: &CreateRequest) -> Result<(), ApiError> {
    let reply = request(
        "POST",
        "/api/events",
        Some(serde_json::to_value(input).map_err(|_| ApiError::invalid())?),
        None,
    )
    .await?;
    if reply["id"].as_str() != Some(&input.id) {
        return Err(ApiError::invalid());
    }
    Ok(())
}
pub async fn event(id: &str) -> Result<Event, ApiError> {
    if !super::valid_id(id) {
        return Err(ApiError {
            status: 404,
            code: "event_not_found".to_owned(),
        });
    }
    let reply = request("GET", &format!("/api/events/{id}"), None, None).await?;
    let event: Event = serde_json::from_value(reply).map_err(|_| ApiError::invalid())?;
    if event.id != id || !event.valid() {
        return Err(ApiError::invalid());
    }
    Ok(event)
}
pub async fn respond(record: &ResponseRecord) -> Result<(), ApiError> {
    let reply = request(
        "POST",
        &format!("/api/events/{}/responses", record.event_id),
        Some(serde_json::to_value(&record.answer).map_err(|_| ApiError::invalid())?),
        Some(("x-response-capability", &record.capability)),
    )
    .await?;
    if reply["event_id"].as_str() != Some(&record.event_id)
        || reply["response_id"].as_str().is_none()
    {
        return Err(ApiError::invalid());
    }
    Ok(())
}
pub async fn responses(id: &str, capability: &str) -> Result<Vec<ResponseView>, ApiError> {
    #[derive(Deserialize)]
    struct Reply {
        responses: Vec<ResponseView>,
    }
    let value = request(
        "GET",
        &format!("/api/events/{id}/responses"),
        None,
        Some(("x-organizer-capability", capability)),
    )
    .await?;
    serde_json::from_value::<Reply>(value)
        .map(|reply| reply.responses)
        .map_err(|_| ApiError::invalid())
}

async fn request(
    method: &str,
    path: &str,
    body: Option<Value>,
    capability: Option<(&str, &str)>,
) -> Result<Value, ApiError> {
    #[cfg(all(feature = "web", target_arch = "wasm32"))]
    {
        use wasm_bindgen::{JsCast, closure::Closure};
        use wasm_bindgen_futures::JsFuture;
        let window = web_sys::window().ok_or_else(ApiError::network)?;
        let options = web_sys::RequestInit::new();
        options.set_method(method);
        options.set_credentials(web_sys::RequestCredentials::SameOrigin);
        options.set_cache(web_sys::RequestCache::NoStore);
        let abort = web_sys::AbortController::new().map_err(|_| ApiError::network())?;
        options.set_signal(Some(&abort.signal()));
        if let Some(body) = body {
            options.set_body(&body.to_string().into());
        }
        let request = web_sys::Request::new_with_str_and_init(path, &options)
            .map_err(|_| ApiError::network())?;
        if method == "POST" {
            request
                .headers()
                .set("Content-Type", "application/json")
                .map_err(|_| ApiError::network())?;
        }
        if let Some((name, value)) = capability {
            request
                .headers()
                .set(name, value)
                .map_err(|_| ApiError::network())?;
        }
        let callback = Closure::<dyn FnMut()>::new(move || abort.abort());
        let timer = window
            .set_timeout_with_callback_and_timeout_and_arguments_0(
                callback.as_ref().unchecked_ref(),
                30_000,
            )
            .map_err(|_| ApiError::network())?;
        struct Timeout {
            window: web_sys::Window,
            timer: i32,
            _callback: Closure<dyn FnMut()>,
        }
        impl Drop for Timeout {
            fn drop(&mut self) {
                self.window.clear_timeout_with_handle(self.timer);
            }
        }
        let _timeout = Timeout {
            window: window.clone(),
            timer,
            _callback: callback,
        };
        let response: web_sys::Response = JsFuture::from(window.fetch_with_request(&request))
            .await
            .map_err(|_| ApiError::network())?
            .dyn_into()
            .map_err(|_| ApiError::invalid())?;
        let status = response.status();
        let text = JsFuture::from(response.text().map_err(|_| ApiError::invalid())?)
            .await
            .map_err(|_| ApiError::network())?
            .as_string()
            .ok_or_else(ApiError::invalid)?;
        let value: Value = serde_json::from_str(&text).map_err(|_| ApiError::invalid())?;
        if !response.ok() {
            return Err(ApiError {
                status,
                code: value["error"]["code"]
                    .as_str()
                    .unwrap_or("request_failed")
                    .to_owned(),
            });
        }
        Ok(value)
    }
    #[cfg(not(all(feature = "web", target_arch = "wasm32")))]
    {
        let _ = (method, path, body, capability);
        Err(ApiError::network())
    }
}
