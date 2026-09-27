//! Task 049: the stable Rust adapter contracts behind generated OMS service
//! APIs.
//!
//! Generated Rust service wrappers (Task 048) forward every typed operation
//! to a caller-supplied adapter. Those adapter traits used to be declared
//! inside every generated `service_api` module; they now live here, once,
//! and each generated wrapper re-exports them:
//!
//! ```text
//! pub use ::ams_gra_oms_runtime_api::{PublishAdapter, SubscribeAdapter};
//! ```
//!
//! so `service_api::service_api::{PublishAdapter, SubscribeAdapter}` keeps
//! resolving for applications, and one runtime implementation (for example
//! `ams-gra-oms-runtime-rust`) can serve every generated service.
//!
//! This crate is intentionally tiny: no dependency, no allocation, no
//! executor, and no `Send`/`Sync`/`'static` requirement. A concrete runtime
//! adds whatever bounds it genuinely needs in its own `impl`.

#![no_std]

/// The runtime hook behind every generated `publish` operation.
///
/// Receives the resolved global message identity (namespace and local
/// name, unformatted), the endpoint's authored topic, and a typed
/// payload. The identity is required because distinct global messages
/// may share one payload type and one topic. `Output` is the adapter's
/// own result/error type.
pub trait PublishAdapter<P> {
    type Output;
    fn publish(
        &mut self,
        message_namespace: &'static str,
        message_name: &'static str,
        topic: &'static str,
        value: &P,
    ) -> Self::Output;
}

/// The runtime hook behind every generated `subscribe` operation.
///
/// Receives the resolved message identity (namespace and local name,
/// unformatted), the authored topic, the authored subscription group
/// if any, and a handler for payload `P`. `Output` is the adapter's
/// own result, error, or subscription-token type.
///
/// The handler type is a trait parameter so an implementation may require
/// any bound it genuinely needs (for example `H: FnMut(&P) + Send + 'static`)
/// without the generated service naming it.
pub trait SubscribeAdapter<P, H> {
    type Output;
    fn subscribe(
        &mut self,
        message_namespace: &'static str,
        message_name: &'static str,
        topic: &'static str,
        subscription_group: Option<&'static str>,
        handler: H,
    ) -> Self::Output;
}
