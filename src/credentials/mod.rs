//! Credential provider boundaries for `ws clip` (User Story 8, User Story 9): the password-store
//! boundary, the mapping configuration that resolves `NAMESPACE ITEM` to a source, and the
//! clipboard destination.

pub mod clip_mapping;
pub mod clipboard;
pub mod pass_provider;
