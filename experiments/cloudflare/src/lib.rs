//! Fixed synthetic probes, not an authentication or event API.
#[allow(dead_code)]
#[path = "../../../src/domain.rs"]
mod domain;

/// Check production domain validation without copying it into JavaScript.
#[unsafe(no_mangle)]
pub extern "C" fn domain_probe() -> u32 {
    let input = domain::NewEventInput {
        name: "synthetic probe".into(),
        organizer_note: None,
        time_zone: "Asia/Tokyo".into(),
        candidates: vec![domain::CandidateInput {
            local_date: "2026-10-01".into(),
            local_time: "19:00".into(),
        }],
    };
    let mut invalid = input.clone();
    invalid.name.clear();
    let mut bad_zone = input.clone();
    bad_zone.time_zone = "Not/AZone".into();
    u32::from(input.normalized_and_validated().is_ok())
        | (u32::from(invalid.normalized_and_validated().is_err()) << 1)
        | (u32::from(bad_zone.normalized_and_validated().is_err()) << 2)
}

/// Deterministic fixture only: production must use a fresh random salt.
#[unsafe(no_mangle)]
pub extern "C" fn argon2_probe(wrong_password: u32) -> u32 {
    use argon2::{Algorithm, Argon2, Params, Version};
    let params = Params::new(19_456, 2, 1, Some(32)).unwrap();
    let context = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut output = [0u8; 32];
    let password: &[u8] = if wrong_password == 0 {
        b"synthetic probe password"
    } else {
        b"different probe password"
    };
    context
        .hash_password_into(password, b"synthetic-salt-16", &mut output)
        .unwrap();
    // Fingerprint is only for comparing fixture computations, never login verification.
    u32::from_le_bytes(output[..4].try_into().unwrap())
}
