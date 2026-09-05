//! Synchronous JSON bridge, called only by the bundled Worker.
#[allow(dead_code)]
#[path = "../../../src/domain.rs"]
mod domain;
use argon2::{Algorithm, Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier, Version};
use serde_json::{Value, json};

fn handle(value: Value) -> Result<Value, String> {
    let op = value["op"].as_str().ok_or("operation required")?;
    let input = value["input"].clone();
    macro_rules! validate {
        ($ty:ty) => {{
            let v: $ty = serde_json::from_value(input).map_err(|_| "入力を確認してください。")?;
            serde_json::to_value(v.normalized_and_validated().map_err(|e| e.to_string())?).map_err(|_| "serialization failed".into())
        }};
    }
    match op {
        "event" => validate!(domain::NewEventInput),
        "answer" => validate!(domain::NewAvailabilityResponseInput),
        "comment" => validate!(domain::NewResponseCommentInput),
        "organizer" => validate!(domain::OrganizerSummaryInput),
        "decision" => validate!(domain::OrganizerDecisionInput),
        "continuation" => validate!(domain::EventContinuationCreateInput),
        "plan" => validate!(domain::EventContinuationPlanInput),
        "trace" => validate!(domain::AccountEventTraceInput),
        "register" => {
            let v: domain::AccountRegistrationInput = serde_json::from_value(input).map_err(|_| "入力を確認してください。")?;
            let p = v.prepare().map_err(|e| e.to_string())?;
            Ok(json!({"login_id": p.login_id, "password": p.password}))
        }
        "login" => {
            let v: domain::AccountLoginInput = serde_json::from_value(input).map_err(|_| "入力を確認してください。")?;
            let p = v.prepare().map_err(|e| e.to_string())?;
            Ok(json!({"login_id": p.login_id, "password": p.password}))
        }
        "hash" | "verify" => {
            let password = input["password"].as_str().ok_or("password required")?;
            if password.len() > 512 { return Err("password too long".into()); }
            let context = Argon2::new(Algorithm::Argon2id, Version::V0x13, Params::new(19_456, 2, 1, Some(32)).unwrap());
            if op == "hash" {
                let salt: Vec<u8> = serde_json::from_value(input["salt"].clone()).map_err(|_| "salt required")?;
                if salt.len() != 16 { return Err("salt length".into()); }
                Ok(json!(context.hash_password_with_salt(password.as_bytes(), &salt).map_err(|_| "hash failed")?.to_string()))
            } else {
                let hash = PasswordHash::new(input["phc"].as_str().ok_or("hash required")?).map_err(|_| "invalid hash")?;
                Ok(json!(context.verify_password(password.as_bytes(), &hash).is_ok()))
            }
        }
        "facts" => {
            let mut s: domain::OrganizerEventSummary = serde_json::from_value(input).map_err(|_| "invalid summary")?;
            domain::derive_candidate_summary_facts(s.response_count, &mut s.candidates);
            serde_json::to_value(s).map_err(|_| "serialization failed".into())
        }
        _ => Err("unknown operation".into()),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn allocate(len: u32) -> *mut u8 {
    if len > 262_144 { return std::ptr::null_mut(); }
    Box::into_raw(vec![0u8; len as usize].into_boxed_slice()) as *mut u8
}
/// # Safety
/// Supply a live allocation from this module with its exact length, once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn release(ptr: *mut u8, len: u32) {
    unsafe { drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, len as usize))); }
}
/// # Safety
/// Supply a live allocation of `len` bytes for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dispatch(ptr: *const u8, len: u32) -> u64 {
    let input = unsafe { std::slice::from_raw_parts(ptr, len as usize) };
    let result = serde_json::from_slice(input).map_err(|_| "invalid JSON".into()).and_then(handle);
    let output = match result { Ok(value) => json!({"ok":value}), Err(error) => json!({"error":error}) };
    let bytes = serde_json::to_vec(&output).unwrap().into_boxed_slice();
    let len = bytes.len() as u64;
    let ptr = Box::into_raw(bytes) as *mut u8 as u64;
    (ptr << 32) | len
}
