//! Flask-RESTful and Flask-RESTX, which is a fork of it with the same API, call the methods of a
//! `Resource` named after HTTP methods: its `dispatch_request` calls the one named after the method
//! of the request, and reads its class attribute `method_decorators`, the decorators it applies to
//! that method. A `Resource` is a `MethodView` of Flask, which the rules of Flask follow when the
//! package is installed; this also covers a project that is checked without it, with the class
//! attributes of a view that Flask reads. The `resource`
//! decorator of an `Api` of Flask-RESTful, and the `route` decorator of an `Api` or a `Namespace` of
//! Flask-RESTX, add the class they decorate as a resource. The `representation` decorator of an `Api`
//! registers the function it decorates to make the responses of a media type, and the `errorhandler`
//! decorator of an `Api` or a `Namespace` of Flask-RESTX, and its `documentation` decorator of an
//! `Api`, register the function they decorate. The instances are those of a variable that is set to
//! one at the top level of a module, or in the function that the decorated definition is defined in,
//! of a parameter of that function that is annotated as one, and, for a `Namespace`, of a variable at
//! the top level that is set to what the `namespace` method of an `Api` of Flask-RESTX makes.

use crate::definitions::{Definition, Kind};

use super::flask::FLASK_VIEW_ATTRIBUTES;
use super::{Call, Context, HTTP_METHODS, Instances, Rules, Settings, has_any};

/// The classes of the resources, as `flask_restful` and `flask_restx` give them out, and where they
/// are defined.
const RESTFUL_RESOURCES: [&str; 3] = [
    "flask_restful.Resource",
    "flask_restx.Resource",
    "flask_restx.resource.Resource",
];
/// The class attribute of a resource that its `dispatch_request` reads.
const RESTFUL_METHOD_DECORATORS: &str = "method_decorators";
/// The classes of the APIs and their namespaces, as `flask_restful` and `flask_restx` give them out,
/// and where they are defined.
const RESTFUL_APIS: [&str; 5] = [
    "flask_restful.Api",
    "flask_restx.Api",
    "flask_restx.api.Api",
    "flask_restx.Namespace",
    "flask_restx.namespace.Namespace",
];
/// The method of an `Api` of Flask-RESTX that makes a namespace.
const RESTX_NAMESPACE: &str = "namespace";
/// The decorators of an `Api` or a `Namespace` that add the class they decorate as a resource.
const RESTFUL_CLASS_DECORATORS: [&str; 2] = ["resource", "route"];
/// The decorators of an `Api` or a `Namespace` that register the function they decorate.
const RESTFUL_DECORATORS: [&str; 3] = ["representation", "errorhandler", "documentation"];

/// The rules of Flask-RESTful and Flask-RESTX.
#[derive(Default)]
pub(super) struct FlaskRestful {
    /// The APIs and their namespaces.
    apis: Instances,
    /// The variables at the top level that are set to what a method `namespace` makes, each by its
    /// module, its dotted name, and what the method is of, as it is written.
    namespaces: Vec<(usize, String, String)>,
}

impl Rules for FlaskRestful {
    fn new(context: &Context, _settings: &Settings) -> Self {
        let mut locals = context.local_instances(&RESTFUL_APIS);
        locals.extend(context.annotated_parameters(&RESTFUL_APIS));
        Self {
            apis: Instances {
                locals,
                ..Instances::default()
            },
            ..Self::default()
        }
    }

    fn variable_call(
        &mut self,
        context: &Context,
        module: usize,
        variable: &Definition,
        call: &Call,
    ) {
        let dotted = context.dotted_name(module, &variable.qualified_name);
        if has_any(call.resolved(context, module), &RESTFUL_APIS) {
            self.apis.variables.insert(dotted);
        } else if let Some(api) = call
            .written
            .strip_suffix(RESTX_NAMESPACE)
            .and_then(|api| api.strip_suffix('.'))
        {
            self.namespaces.push((module, dotted, api.to_string()));
        }
    }

    /// Learn which variables are set to a namespace of an `Api` of Flask-RESTX, now that every `Api` is
    /// known.
    fn finish(&mut self, context: &Context) {
        for (module, namespace, api) in std::mem::take(&mut self.namespaces) {
            let is_api = self
                .apis
                .variables
                .contains(&context.dotted_name(module, &api))
                || context
                    .hierarchy
                    .resolve(module, &api)
                    .iter()
                    .any(|name| self.apis.variables.contains(name));
            if is_api {
                self.apis.variables.insert(namespace);
            }
        }
    }

    /// Check whether Flask-RESTful or Flask-RESTX calls or adds a definition by itself.
    fn uses(&self, context: &Context, definition: &Definition, module: usize) -> bool {
        match definition.kind {
            Kind::Method => {
                if let Some(class) = context.class_name(definition, module)
                    && HTTP_METHODS.contains(&definition.name.as_str())
                    && context.inherits(module, class, &RESTFUL_RESOURCES)
                {
                    return true;
                }
            }
            Kind::Attribute => {
                let name = definition.name.as_str();
                return (name == RESTFUL_METHOD_DECORATORS
                    || FLASK_VIEW_ATTRIBUTES.contains(&name))
                    && context
                        .class_name(definition, module)
                        .is_some_and(|class| context.inherits(module, class, &RESTFUL_RESOURCES));
            }
            Kind::Class => {
                return context.decorated_by(
                    definition,
                    module,
                    &self.apis,
                    &[&RESTFUL_CLASS_DECORATORS],
                );
            }
            _ => {}
        }
        context.registers(definition, module, &self.apis, &[&RESTFUL_DECORATORS])
    }
}
