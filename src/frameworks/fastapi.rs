//! FastAPI calls the functions that the decorators of an application or a router register, such as
//! `app.get` or `router.websocket`: of a variable that is set to `FastAPI()` or `APIRouter()`, or
//! to an instance of a subclass of them, at the top level of a module, or in the function that the
//! decorated function is defined in.

use crate::definitions::Definition;

use super::{FrameworkRules, HTTP_METHODS};

/// The classes of FastAPI's applications and routers, as `fastapi` gives them out, and where they are
/// defined.
pub(super) const FASTAPI_ROUTERS: [&str; 4] = [
    "fastapi.FastAPI",
    "fastapi.applications.FastAPI",
    "fastapi.APIRouter",
    "fastapi.routing.APIRouter",
];
/// The methods of an application or a router that register the function they decorate, besides
/// those named after HTTP methods.
pub(super) const FASTAPI_DECORATORS: [&str; 5] = [
    "api_route",
    "websocket",
    "on_event",
    "middleware",
    "exception_handler",
];

impl FrameworkRules<'_> {
    /// Check whether FastAPI calls a definition by itself.
    pub(super) fn fastapi_uses(&self, definition: &Definition, module: usize) -> bool {
        self.registers(
            definition,
            module,
            &self.fastapi_routers,
            &[&HTTP_METHODS, &FASTAPI_DECORATORS],
        )
    }
}
