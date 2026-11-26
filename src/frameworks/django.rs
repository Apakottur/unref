//! Django reads the settings of its settings modules: the names at the top level that are all
//! uppercase, and not the others. Its settings modules are those that `DJANGO_SETTINGS_MODULE` is
//! set to with `os.environ.setdefault`, as `startproject` writes in `manage.py`, `wsgi.py` and
//! `asgi.py`, those that the `settings` option of the `frameworks.django` table names, and the
//! modules that they star import, such as a `base.py` of split settings. It reads `urlpatterns` and
//! the error handlers, such as `handler404`, from the module that the `ROOT_URLCONF` setting names,
//! and `urlpatterns` and `app_name` from each module that `include` names by a string. The server
//! loads the application that `get_wsgi_application()` or `get_asgi_application()` makes, by a
//! dotted path. Django makes the subclasses of `AppConfig` of the installed applications, reads
//! their attributes such as `name`, and calls their `ready` method. Its migration commands load the
//! class named `Migration` of a migration module, a subclass of `Migration`, and read its
//! attributes, such as `operations`.
//!
//! The metaclass of a model, a subclass of `Model`, reads its fields, the attributes set to an
//! instance of a field class, and its managers, which Django's ORM, migrations, admin and forms then
//! use. Django uses the first manager of a model as its default manager, and calls its
//! `get_queryset` method for each query. It reads the options of the inner `Meta` class of a model,
//! and raises an error for any other attribute, and calls the `get_absolute_url` method of a model
//! in the admin and in `redirect`. The metaclass of a form reads its fields, and that of a
//! `ModelForm` reads its inner `Meta` class. A form calls its `clean_<field>` methods to clean each
//! field. Forms, widgets and the classes of the admin read the attributes of their inner `Media`
//! class. A class-based view, a subclass of `View`, calls the method named by the HTTP method of a
//! request, such as `get`. The `FilterSet` of `django-filter` reads its filters, the attributes set
//! to an instance of a filter class, and its inner `Meta` class.
//!
//! `manage.py` runs the class named `Command` of a module of a `management/commands` package whose
//! name does not start with `_`, a subclass of `BaseCommand`, and calls the methods of a command such
//! as `handle`. Django loads the template libraries of the `templatetags` packages by their
//! `register` variable, a `Library()`, whose decorators, such as `filter` and `simple_tag`, register
//! the tags and filters that templates use by their names. A signal calls the receivers that its
//! `receiver` decorator connects.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use rustc_hash::{FxHashMap, FxHashSet};

use crate::definitions::{Definition, Kind};
use crate::modules::resolve;

use super::{
    Call, Context, HTTP_METHODS, Instances, Rules, Settings, has_any, is_top_level, is_uppercase,
};

/// The environment variable that names the settings module of Django, and `os.environ.setdefault`,
/// which `startproject` sets it with.
const DJANGO_SETTINGS_VARIABLE: &str = "DJANGO_SETTINGS_MODULE";
const OS_ENVIRON_SETDEFAULT: [&str; 1] = ["os.environ.setdefault"];
/// The setting that names the module of the root URLs.
const DJANGO_ROOT_URLCONF: &str = "ROOT_URLCONF";
/// `include`, which names a module of URLs, as `django.urls` gives it out, where it is defined, and as
/// `django.conf.urls` gave it out before Django 4.
const DJANGO_INCLUDES: [&str; 3] = [
    "django.urls.include",
    "django.urls.conf.include",
    "django.conf.urls.include",
];
/// The variables that Django reads from a module of URLs, and those that it reads from the module of
/// the root URLs only, or from an included one only.
const DJANGO_URLS_VARIABLES: [&str; 1] = ["urlpatterns"];
const DJANGO_ROOT_URLS_VARIABLES: [&str; 4] =
    ["handler400", "handler403", "handler404", "handler500"];
const DJANGO_INCLUDED_URLS_VARIABLES: [&str; 1] = ["app_name"];
/// The functions that make the application that a server loads.
const DJANGO_APPLICATIONS: [&str; 2] = [
    "django.core.wsgi.get_wsgi_application",
    "django.core.asgi.get_asgi_application",
];
/// `AppConfig`, as `django.apps` gives it out, and where it is defined, with the attributes and
/// methods of its subclasses that Django reads and calls.
const DJANGO_APP_CONFIGS: [&str; 2] = ["django.apps.AppConfig", "django.apps.config.AppConfig"];
const DJANGO_APP_CONFIG_ATTRIBUTES: [&str; 6] = [
    "name",
    "label",
    "verbose_name",
    "path",
    "default",
    "default_auto_field",
];
const DJANGO_APP_CONFIG_METHODS: [&str; 1] = ["ready"];
/// `Migration`, as `django.db.migrations` gives it out, and where it is defined, with the attributes
/// of its subclasses that Django reads.
const DJANGO_MIGRATIONS: [&str; 2] = [
    "django.db.migrations.Migration",
    "django.db.migrations.migration.Migration",
];
/// The name of the class that Django loads from a migration module.
const DJANGO_MIGRATION_CLASS: &str = "Migration";
const DJANGO_MIGRATION_ATTRIBUTES: [&str; 6] = [
    "operations",
    "dependencies",
    "run_before",
    "replaces",
    "initial",
    "atomic",
];

/// `Model`, as `django.db.models` gives it out, and where it is defined, and the abstract models of
/// users of `django.contrib.auth`, which a model inherits from without `Model` when Django is not
/// installed.
const DJANGO_MODELS: [&str; 5] = [
    "django.db.models.Model",
    "django.db.models.base.Model",
    "django.contrib.auth.models.AbstractUser",
    "django.contrib.auth.models.AbstractBaseUser",
    "django.contrib.auth.base_user.AbstractBaseUser",
];
/// The ends of the names of Django's field classes, of models and of forms, such as
/// `django.db.models.CharField` and `django.contrib.contenttypes.fields.GenericForeignKey`.
const DJANGO_FIELDS: [&str; 5] = [
    "Field",
    "ForeignKey",
    "ForeignObject",
    "GenericRelation",
    "CompositePrimaryKey",
];
/// `Manager`, as `django.db.models` gives it out, and where it is defined, and `QuerySet`, whose
/// `as_manager` makes a manager.
const DJANGO_MANAGERS: [&str; 3] = [
    "django.db.models.Manager",
    "django.db.models.manager.Manager",
    "django.db.models.manager.BaseManager",
];
const DJANGO_QUERY_SETS: [&str; 2] = [
    "django.db.models.QuerySet",
    "django.db.models.query.QuerySet",
];
const DJANGO_AS_MANAGER: &str = ".as_manager";
/// The methods of a manager and of a model that Django calls.
const DJANGO_MANAGER_METHODS: [&str; 1] = ["get_queryset"];
const DJANGO_MODEL_METHODS: [&str; 1] = ["get_absolute_url"];
/// The forms, as `django.forms` gives them out, and where they are defined, and the model forms,
/// which read their `Meta` class.
const DJANGO_FORMS: [&str; 6] = [
    "django.forms.BaseForm",
    "django.forms.forms.BaseForm",
    "django.forms.Form",
    "django.forms.forms.Form",
    "django.forms.ModelForm",
    "django.forms.models.ModelForm",
];
const DJANGO_MODEL_FORMS: [&str; 2] = ["django.forms.ModelForm", "django.forms.models.ModelForm"];
/// The prefix of the methods of a form that clean a field.
const DJANGO_CLEAN_PREFIX: &str = "clean_";
/// The classes besides forms that read their `Media` class: widgets and the classes of the admin.
const DJANGO_MEDIA_CLASSES: [&str; 8] = [
    "django.forms.Widget",
    "django.forms.widgets.Widget",
    "django.contrib.admin.ModelAdmin",
    "django.contrib.admin.options.BaseModelAdmin",
    "django.contrib.admin.TabularInline",
    "django.contrib.admin.StackedInline",
    "django.contrib.admin.options.InlineModelAdmin",
    "django.contrib.admin.options.ModelAdmin",
];
/// The inner classes that Django reads.
const DJANGO_META: &str = "Meta";
const DJANGO_MEDIA: &str = "Media";
/// `View`, as `django.views` gives it out, and where it is defined, the package of its generic
/// views, and the HTTP methods that it dispatches to.
const DJANGO_VIEWS: [&str; 2] = ["django.views.View", "django.views.generic.base.View"];
const DJANGO_GENERIC_VIEWS: &str = "django.views.generic";
const DJANGO_VIEW_ENDS: [&str; 1] = ["View"];
/// The package of `django-filter`, with the ends of the names of its filter sets and its filters.
const DJANGO_FILTER: &str = "django_filters";
const DJANGO_FILTER_SETS: [&str; 1] = ["FilterSet"];
const DJANGO_FILTERS: [&str; 1] = ["Filter"];

/// `BaseCommand`, as `django.core.management` gives it out, and where it and its subclasses that make
/// commands of applications and labels are defined, with the attributes and methods of a command that
/// Django reads and calls.
const DJANGO_COMMANDS: [&str; 4] = [
    "django.core.management.BaseCommand",
    "django.core.management.base.BaseCommand",
    "django.core.management.base.AppCommand",
    "django.core.management.base.LabelCommand",
];
const DJANGO_COMMAND_ATTRIBUTES: [&str; 8] = [
    "help",
    "output_transaction",
    "requires_migrations_checks",
    "requires_system_checks",
    "stealth_options",
    "suppressed_base_arguments",
    "missing_args_message",
    "label",
];
const DJANGO_COMMAND_METHODS: [&str; 5] = [
    "handle",
    "add_arguments",
    "create_parser",
    "handle_app_config",
    "handle_label",
];
/// The name of the class that `manage.py` runs from a module of commands, and the packages that hold
/// those modules.
const DJANGO_COMMAND_CLASS: &str = "Command";
const DJANGO_COMMANDS_PACKAGES: [&str; 2] = ["management", "commands"];
/// `Library`, as `django.template` gives it out, and where it is defined, with the methods of a
/// library that register a tag or filter, and the variable that Django loads a library by.
const DJANGO_LIBRARIES: [&str; 2] = ["django.template.Library", "django.template.library.Library"];
const DJANGO_LIBRARY_METHODS: [&str; 5] = [
    "filter",
    "simple_tag",
    "simple_block_tag",
    "inclusion_tag",
    "tag",
];
const DJANGO_LIBRARY_VARIABLE: &str = "register";
/// `receiver`, as `django.dispatch` gives it out, and where it is defined.
const DJANGO_RECEIVERS: [&str; 2] = [
    "django.dispatch.receiver",
    "django.dispatch.dispatcher.receiver",
];

/// The rules of Django, with the modules that it reads by their names, by their indices.
pub(super) struct Django {
    /// The settings modules.
    settings: FxHashSet<usize>,
    /// The modules of the root URLs, which `ROOT_URLCONF` names.
    root_urls: FxHashSet<usize>,
    /// The modules of URLs that `include` names.
    included_urls: FxHashSet<usize>,
    /// The template libraries.
    libraries: Instances,
}

impl Rules for Django {
    fn new(context: &Context, settings: &Settings) -> Self {
        django_modules(context, settings.django_settings)
    }

    fn variable_call(
        &mut self,
        context: &Context,
        module: usize,
        variable: &Definition,
        call: &Call,
    ) {
        if is_top_level(variable) && has_any(call.resolved(context, module), &DJANGO_LIBRARIES) {
            let library = context.dotted_name(module, &variable.qualified_name);
            self.libraries.variables.insert(library);
        }
    }

    /// Check whether Django or a server reads or calls a definition by itself.
    fn uses(&self, context: &Context, definition: &Definition, module: usize) -> bool {
        let name = definition.name.as_str();
        if let Some(class) = context.class_name(definition, module) {
            return uses_member(context, definition, module, class);
        }
        if definition.kind == Kind::Class {
            return definition.owner.is_none()
                && (context.inherits(module, name, &DJANGO_APP_CONFIGS)
                    || (name == DJANGO_MIGRATION_CLASS
                        && context.inherits(module, name, &DJANGO_MIGRATIONS))
                    || (name == DJANGO_COMMAND_CLASS
                        && is_command_module(context.path(module))
                        && context.inherits(module, name, &DJANGO_COMMANDS)));
        }
        if definition.kind.is_function() {
            return context.is_decorated(definition, module, &DJANGO_RECEIVERS)
                || context.registers(
                    definition,
                    module,
                    &self.libraries,
                    &[&DJANGO_LIBRARY_METHODS],
                );
        }
        if definition.kind != Kind::Variable || definition.owner.is_some() {
            return false;
        }
        (self.settings.contains(&module) && is_uppercase(name))
            || ((self.root_urls.contains(&module) || self.included_urls.contains(&module))
                && DJANGO_URLS_VARIABLES.contains(&name))
            || (self.root_urls.contains(&module) && DJANGO_ROOT_URLS_VARIABLES.contains(&name))
            || (self.included_urls.contains(&module)
                && DJANGO_INCLUDED_URLS_VARIABLES.contains(&name))
            || (name == DJANGO_LIBRARY_VARIABLE
                && self
                    .libraries
                    .variables
                    .contains(&context.dotted_name(module, name)))
            || definition
                .call
                .as_ref()
                .is_some_and(|call| context.resolves_to(module, call, &DJANGO_APPLICATIONS))
    }
}

/// Check whether Django reads or calls a definition of a class of a module, given by its name in the
/// module.
fn uses_member(context: &Context, definition: &Definition, module: usize, class: &str) -> bool {
    let name = definition.name.as_str();
    match definition.kind {
        Kind::Class => {
            (name == DJANGO_META && reads_meta(context, module, class))
                || (name == DJANGO_MEDIA && reads_media(context, module, class))
        }
        Kind::Attribute => {
            // An option of an inner `Meta` or `Media` class.
            if let Some((outer, inner)) = class.rsplit_once('.')
                && ((inner == DJANGO_META && reads_meta(context, module, outer))
                    || (inner == DJANGO_MEDIA && reads_media(context, module, outer)))
            {
                return true;
            }
            (DJANGO_APP_CONFIG_ATTRIBUTES.contains(&name)
                && context.inherits(module, class, &DJANGO_APP_CONFIGS))
                || (DJANGO_COMMAND_ATTRIBUTES.contains(&name)
                    && context.inherits(module, class, &DJANGO_COMMANDS))
                || (DJANGO_MIGRATION_ATTRIBUTES.contains(&name)
                    && context.inherits(module, class, &DJANGO_MIGRATIONS))
                || (context.calls_end(definition, module, "django", &DJANGO_FIELDS)
                    && (context.inherits(module, class, &DJANGO_MODELS)
                        || context.inherits(module, class, &DJANGO_FORMS)))
                || (context.calls_end(definition, module, DJANGO_FILTER, &DJANGO_FILTERS)
                    && is_filter_set(context, module, class))
                || (context.inherits(module, class, &DJANGO_MODELS)
                    && is_default_manager(context, definition, module))
        }
        Kind::Method => {
            (DJANGO_APP_CONFIG_METHODS.contains(&name)
                && context.inherits(module, class, &DJANGO_APP_CONFIGS))
                || (DJANGO_COMMAND_METHODS.contains(&name)
                    && context.inherits(module, class, &DJANGO_COMMANDS))
                || (DJANGO_MODEL_METHODS.contains(&name)
                    && context.inherits(module, class, &DJANGO_MODELS))
                || (DJANGO_MANAGER_METHODS.contains(&name)
                    && context.inherits(module, class, &DJANGO_MANAGERS))
                || (name.starts_with(DJANGO_CLEAN_PREFIX)
                    && context.inherits(module, class, &DJANGO_FORMS))
                || (HTTP_METHODS.contains(&name) && is_view(context, module, class))
        }
        _ => false,
    }
}

/// Check whether a file is a module of commands of `manage.py`: a module of a `management/commands`
/// package whose name does not start with `_`.
fn is_command_module(path: &Path) -> bool {
    let mut parents = path.parent().into_iter().flat_map(Path::iter).rev();
    let packages = [parents.next(), parents.next()];
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .is_some_and(|stem| !stem.starts_with('_'))
        && packages
            .into_iter()
            .rev()
            .eq(DJANGO_COMMANDS_PACKAGES.map(|package| Some(OsStr::new(package))))
}

/// Check whether Django or `django-filter` reads the inner `Meta` class of a class of a module: that
/// of a model, a model form or a filter set.
fn reads_meta(context: &Context, module: usize, class: &str) -> bool {
    context.inherits(module, class, &DJANGO_MODELS)
        || context.inherits(module, class, &DJANGO_MODEL_FORMS)
        || is_filter_set(context, module, class)
}

/// Check whether Django reads the inner `Media` class of a class of a module: that of a form, a
/// widget or a class of the admin.
fn reads_media(context: &Context, module: usize, class: &str) -> bool {
    context.inherits(module, class, &DJANGO_FORMS)
        || context.inherits(module, class, &DJANGO_MEDIA_CLASSES)
}

/// Check whether a class of a module is a filter set of `django-filter`.
fn is_filter_set(context: &Context, module: usize, class: &str) -> bool {
    context.inherits_end(module, class, DJANGO_FILTER, &DJANGO_FILTER_SETS)
}

/// Check whether a class of a module is a class-based view.
fn is_view(context: &Context, module: usize, class: &str) -> bool {
    context.inherits(module, class, &DJANGO_VIEWS)
        || context.inherits_end(module, class, DJANGO_GENERIC_VIEWS, &DJANGO_VIEW_ENDS)
}

/// Check whether an attribute of a model of a module is its default manager: the first attribute of
/// the model that is set to a manager.
fn is_default_manager(context: &Context, attribute: &Definition, module: usize) -> bool {
    is_manager(context, attribute, module)
        && !context
            .class_attributes(attribute, module)
            .any(|other| other.line < attribute.line && is_manager(context, other, module))
}

/// Check whether an attribute of a module is set to a manager: an instance of `Manager` or of a
/// subclass of it, or what `as_manager` of a `QuerySet` makes.
fn is_manager(context: &Context, attribute: &Definition, module: usize) -> bool {
    attribute.call.as_ref().is_some_and(|call| {
        context.resolves_to(module, call, &DJANGO_MANAGERS)
            || call
                .strip_suffix(DJANGO_AS_MANAGER)
                .is_some_and(|query_set| context.resolves_to(module, query_set, &DJANGO_QUERY_SETS))
    })
}

/// Find the modules that Django reads by their names, given the settings modules that are given by
/// their resolved paths.
fn django_modules(context: &Context, given: &FxHashSet<PathBuf>) -> Django {
    let (files, module_names, hierarchy) = (context.files, context.module_names, context.hierarchy);
    let mut by_name: FxHashMap<&str, Vec<usize>> = FxHashMap::default();
    for (module, name) in module_names.iter().enumerate() {
        by_name.entry(name.name.as_str()).or_default().push(module);
    }
    let modules = |name: &str| by_name.get(name).into_iter().flatten().copied();

    let mut pending: Vec<usize> = Vec::new();
    let mut included_urls = FxHashSet::default();
    for (module, file) in files.iter().enumerate() {
        if !given.is_empty() && given.contains(&resolve(file.path)) {
            pending.push(module);
        }
        for (called, arguments) in &file.hints.string_calls {
            let resolves_to = |names: &[&str]| hierarchy.resolves_to(module, called, names);
            match arguments.as_slice() {
                [variable, settings, ..]
                    if variable == DJANGO_SETTINGS_VARIABLE
                        && resolves_to(&OS_ENVIRON_SETDEFAULT) =>
                {
                    pending.extend(modules(settings));
                }
                [urls, ..] if resolves_to(&DJANGO_INCLUDES) => included_urls.extend(modules(urls)),
                _ => {}
            }
        }
    }

    // The modules that a settings module star imports give out their settings through it.
    let mut settings = FxHashSet::default();
    while let Some(module) = pending.pop() {
        if !settings.insert(module) {
            continue;
        }
        for import in &files[module].references.imports {
            if import.name == "*"
                && let Some(name) =
                    module_names[module].resolve(import.module.as_deref(), import.level)
            {
                pending.extend(modules(&name));
            }
        }
    }

    let mut root_urls = FxHashSet::default();
    for &module in &settings {
        for definition in files[module].definitions {
            if definition.kind == Kind::Variable
                && is_top_level(definition)
                && definition.name == DJANGO_ROOT_URLCONF
                && let Some(urls) = &definition.string
            {
                root_urls.extend(modules(urls));
            }
        }
    }
    Django {
        settings,
        root_urls,
        included_urls,
        libraries: Instances::default(),
    }
}
