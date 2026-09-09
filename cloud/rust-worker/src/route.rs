//! HTTP route boundary.
//!
//! Cross-cutting request policy is implemented in [`crate::policy`]; the
//! dispatch table remains in the worker entry point until the next mechanical
//! extraction so route behavior can be compared independently.
