//! FastAPI calls the functions that the decorators of an application or a router register, such as
//! `app.get` or `router.websocket`: of a variable that is set to `FastAPI()` or `APIRouter()`, or
//! to an instance of a subclass of them, at the top level of a module, or in the function that the
//! decorated function is defined in.

use crate::definitions::Definition;

use super::{Call, Context, HTTP_METHODS, Instances, Rules, Settings, has_any};

/// The classes of FastAPI's applications and routers, as `fastapi` gives them out, and where they are
/// defined.
const FASTAPI_ROUTERS: [&str; 4] = [
    "fastapi.FastAPI",
    "fastapi.applications.FastAPI",
    "fastapi.APIRouter",
    "fastapi.routing.APIRouter",
];
/// The methods of an application or a router that register the function they decorate, besides
/// those named after HTTP methods. `route` and `websocket_route` are Starlette's.
const FASTAPI_DECORATORS: [&str; 7] = [
    "api_route",
    "websocket",
    "on_event",
    "middleware",
    "exception_handler",
    "route",
    "websocket_route",
];
/// The methods of the webhooks of an application, as `app.webhooks.post`, which register the
/// function they decorate as the documentation of a webhook.
const FASTAPI_WEBHOOKS: [&str; 8] = [
    "webhooks.get",
    "webhooks.put",
    "webhooks.post",
    "webhooks.delete",
    "webhooks.options",
    "webhooks.head",
    "webhooks.patch",
    "webhooks.trace",
];

/// The rules of FastAPI.
pub(super) struct Fastapi {
    /// The applications and routers.
    routers: Instances,
}

impl Rules for Fastapi {
    fn new(context: &Context, _settings: &Settings) -> Self {
        Self {
            routers: Instances {
                locals: context.local_instances(&FASTAPI_ROUTERS),
                ..Instances::default()
            },
        }
    }

    fn variable_call(
        &mut self,
        context: &Context,
        module: usize,
        variable: &Definition,
        call: &Call,
    ) {
        if has_any(call.resolved(context, module), &FASTAPI_ROUTERS) {
            let router = context.dotted_name(module, &variable.qualified_name);
            self.routers.variables.insert(router);
        }
    }

    /// Check whether FastAPI calls a definition by itself.
    fn uses(&self, context: &Context, definition: &Definition, module: usize) -> bool {
        context.registers(
            definition,
            module,
            &self.routers,
            &[&HTTP_METHODS, &FASTAPI_DECORATORS, &FASTAPI_WEBHOOKS],
        )
    }
}
