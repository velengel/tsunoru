//! Small browser capabilities. No script evaluation or browser secrets in URLs.

pub fn local_date() -> Option<String> {
    #[cfg(all(feature = "web", target_arch = "wasm32"))]
    {
        let date = js_sys::Date::new_0();
        Some(format!(
            "{:04}-{:02}-{:02}",
            date.get_full_year(),
            date.get_month() + 1,
            date.get_date()
        ))
    }
    #[cfg(not(all(feature = "web", target_arch = "wasm32")))]
    None
}

pub fn time_zone() -> Option<String> {
    #[cfg(all(feature = "web", target_arch = "wasm32"))]
    {
        let format =
            js_sys::Intl::DateTimeFormat::new(&js_sys::Array::new(), &js_sys::Object::new());
        js_sys::Reflect::get(&format.resolved_options(), &"timeZone".into())
            .ok()?
            .as_string()
    }
    #[cfg(not(all(feature = "web", target_arch = "wasm32")))]
    None
}

pub fn random_key() -> Result<String, String> {
    #[cfg(all(feature = "web", target_arch = "wasm32"))]
    {
        let crypto = web_sys::window()
            .and_then(|window| window.crypto().ok())
            .ok_or_else(|| "安全な送信の準備ができませんでした。".to_owned())?;
        let mut bytes = [0u8; 32];
        crypto
            .get_random_values_with_u8_array(&mut bytes)
            .map_err(|_| "安全な送信の準備ができませんでした。".to_owned())?;
        Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
    }
    #[cfg(not(all(feature = "web", target_arch = "wasm32")))]
    Err("ブラウザーで開いてください。".to_owned())
}

pub fn absolute_url(path: &str) -> String {
    #[cfg(all(feature = "web", target_arch = "wasm32"))]
    if let Some(origin) = web_sys::window().and_then(|window| window.location().origin().ok()) {
        return format!("{origin}{path}");
    }
    path.to_owned()
}

pub fn focus(id: &str) {
    #[cfg(all(feature = "web", target_arch = "wasm32"))]
    {
        use wasm_bindgen::JsCast;
        if let Some(element) = web_sys::window()
            .and_then(|window| window.document())
            .and_then(|document| document.get_element_by_id(id))
            .and_then(|element| element.dyn_into::<web_sys::HtmlElement>().ok())
        {
            let _ = element.focus();
        }
    }
    #[cfg(not(all(feature = "web", target_arch = "wasm32")))]
    let _ = id;
}

pub async fn copy(value: &str) -> bool {
    #[cfg(all(feature = "web", target_arch = "wasm32"))]
    if let Some(window) = web_sys::window() {
        return wasm_bindgen_futures::JsFuture::from(
            window.navigator().clipboard().write_text(value),
        )
        .await
        .is_ok();
    }
    let _ = value;
    false
}

#[derive(Clone, Copy)]
pub struct LocalStore;

impl crate::cloud::Store for LocalStore {
    fn read(&self, key: &str) -> Result<Option<String>, String> {
        #[cfg(all(feature = "web", target_arch = "wasm32"))]
        {
            storage()?.get_item(key).map_err(|_| storage_error())
        }
        #[cfg(not(all(feature = "web", target_arch = "wasm32")))]
        {
            let _ = key;
            Ok(None)
        }
    }

    fn write(&self, key: &str, value: &str) -> Result<(), String> {
        #[cfg(all(feature = "web", target_arch = "wasm32"))]
        {
            let store = storage()?;
            store.set_item(key, value).map_err(|_| storage_error())?;
            if store.get_item(key).map_err(|_| storage_error())?.as_deref() != Some(value) {
                return Err(storage_error());
            }
            Ok(())
        }
        #[cfg(not(all(feature = "web", target_arch = "wasm32")))]
        {
            let _ = (key, value);
            Err(storage_error())
        }
    }

    fn remove(&self, key: &str) -> Result<(), String> {
        #[cfg(all(feature = "web", target_arch = "wasm32"))]
        {
            let store = storage()?;
            store.remove_item(key).map_err(|_| storage_error())?;
            if store.get_item(key).map_err(|_| storage_error())?.is_some() {
                return Err(storage_error());
            }
            Ok(())
        }
        #[cfg(not(all(feature = "web", target_arch = "wasm32")))]
        {
            let _ = key;
            Err(storage_error())
        }
    }
}

pub fn storage_error() -> String {
    "このブラウザーに送信内容を保存できません。サイトの保存を許可してから、もう一度お試しください。送信は始めていません。".to_owned()
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn storage() -> Result<web_sys::Storage, String> {
    web_sys::window()
        .and_then(|window| window.local_storage().ok().flatten())
        .ok_or_else(storage_error)
}
