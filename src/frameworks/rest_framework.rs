//! The metaclass of a serializer of Django REST framework, a subclass of `Serializer`, reads its
//! fields, the attributes set to an instance of a field class or of a serializer, and a
//! `ModelSerializer` reads its inner `Meta` class. A serializer calls its `validate_<field>` methods
//! to validate each field, and a `SerializerMethodField` calls the `get_<field>` method of the field.
//! A view, a subclass of `APIView`, calls the method named by the HTTP method of a request, such as
//! `get`, and reads its attributes that configure it, such as `serializer_class` and
//! `permission_classes`, or a filter backend, such as `filterset_class` and `search_fields`. The
//! routers route requests to the actions of a viewset, such as `list` and `retrieve`, and to its
//! methods that `action` registers.

use crate::definitions::{Definition, Kind};

use super::{Context, HTTP_METHODS, Rules, Settings};

/// The package of Django REST framework, with the ends of the names of its serializers, and of its
/// fields and serializers, which a serializer can have as a field.
const REST_FRAMEWORK: &str = "rest_framework";
const SERIALIZERS: [&str; 1] = ["Serializer"];
const SERIALIZER_FIELDS: [&str; 2] = ["Field", "Serializer"];
/// The inner class that a serializer reads.
const SERIALIZER_META: &str = "Meta";
/// The prefix of the methods of a serializer that validate a field.
const VALIDATE_PREFIX: &str = "validate_";
/// `SerializerMethodField`, as `rest_framework.serializers` gives it out, and where it is defined,
/// and the prefix of the method that it calls by default.
const SERIALIZER_METHOD_FIELDS: [&str; 2] = [
    "rest_framework.serializers.SerializerMethodField",
    "rest_framework.fields.SerializerMethodField",
];
const SERIALIZER_METHOD_PREFIX: &str = "get_";
/// `APIView`, as `rest_framework.views` gives it out, and the packages of the generic views and of the
/// viewsets, all of which are views.
const API_VIEWS: [&str; 1] = ["rest_framework.views.APIView"];
const GENERIC_VIEWS: &str = "rest_framework.generics";
const VIEWSETS: &str = "rest_framework.viewsets";
const VIEW_ENDS: [&str; 2] = ["View", "ViewSet"];
const VIEWSET_ENDS: [&str; 2] = ["ViewSet", "ViewSetMixin"];
/// The attributes of a view that `APIView` and `GenericAPIView` read, and those that the filter
/// backends of Django REST framework and `django-filter` read.
const VIEW_ATTRIBUTES: [&str; 22] = [
    "renderer_classes",
    "parser_classes",
    "authentication_classes",
    "throttle_classes",
    "permission_classes",
    "content_negotiation_class",
    "metadata_class",
    "versioning_class",
    "schema",
    "queryset",
    "serializer_class",
    "lookup_field",
    "lookup_url_kwarg",
    "filter_backends",
    "pagination_class",
    "throttle_scope",
    "search_fields",
    "ordering_fields",
    "ordering",
    "filterset_class",
    "filterset_fields",
    "http_method_names",
];
/// The actions of a viewset that the routers route to.
const VIEWSET_ACTIONS: [&str; 6] = [
    "list",
    "create",
    "retrieve",
    "update",
    "partial_update",
    "destroy",
];
/// The decorator that registers an extra action of a viewset, as `rest_framework.decorators` gives it
/// out.
const ACTIONS: [&str; 1] = ["rest_framework.decorators.action"];

/// The rules of Django REST framework.
pub(super) struct RestFramework;

impl Rules for RestFramework {
    fn new(_context: &Context, _settings: &Settings) -> Self {
        Self
    }

    /// Check whether Django REST framework reads or calls a definition by itself.
    fn uses(&self, context: &Context, definition: &Definition, module: usize) -> bool {
        let Some(class) = context.class_name(definition, module) else {
            return false;
        };
        let name = definition.name.as_str();
        match definition.kind {
            Kind::Class => name == SERIALIZER_META && is_serializer(context, module, class),
            Kind::Attribute => {
                if let Some((serializer, inner)) = class.rsplit_once('.')
                    && inner == SERIALIZER_META
                    && is_serializer(context, module, serializer)
                {
                    return true;
                }
                (is_serializer(context, module, class)
                    && context.calls_end(definition, module, REST_FRAMEWORK, &SERIALIZER_FIELDS))
                    || (VIEW_ATTRIBUTES.contains(&name) && is_view(context, module, class))
            }
            Kind::Method => {
                (is_serializer(context, module, class)
                    && (name.starts_with(VALIDATE_PREFIX)
                        || calls_method(context, definition, module, name)))
                    || (HTTP_METHODS.contains(&name) && is_view(context, module, class))
                    || (VIEWSET_ACTIONS.contains(&name)
                        && context.inherits_end(module, class, VIEWSETS, &VIEWSET_ENDS))
                    || context.is_decorated(definition, module, &ACTIONS)
            }
            _ => false,
        }
    }
}

/// Check whether a class of a module is a serializer.
fn is_serializer(context: &Context, module: usize, class: &str) -> bool {
    context.inherits_end(module, class, REST_FRAMEWORK, &SERIALIZERS)
}

/// Check whether a class of a module is a view.
fn is_view(context: &Context, module: usize, class: &str) -> bool {
    context.inherits(module, class, &API_VIEWS)
        || context.inherits_end(module, class, GENERIC_VIEWS, &VIEW_ENDS)
        || context.inherits_end(module, class, VIEWSETS, &VIEWSET_ENDS)
}

/// Check whether a method of a serializer of a module is the `get_<field>` method that a
/// `SerializerMethodField` of the serializer calls.
fn calls_method(context: &Context, method: &Definition, module: usize, name: &str) -> bool {
    let Some(field) = name.strip_prefix(SERIALIZER_METHOD_PREFIX) else {
        return false;
    };
    context.class_attributes(method, module).any(|attribute| {
        attribute.name == field
            && attribute
                .call
                .as_ref()
                .is_some_and(|call| context.resolves_to(module, call, &SERIALIZER_METHOD_FIELDS))
    })
}
