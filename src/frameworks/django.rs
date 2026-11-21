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

use std::path::PathBuf;

use rustc_hash::{FxHashMap, FxHashSet};

use crate::definitions::{Definition, Kind};
use crate::modules::resolve;

use super::{Context, Rules, Settings, is_top_level};

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

/// The rules of Django, with the modules that it reads by their names, by their indices.
pub(super) struct Django {
    /// The settings modules.
    settings: FxHashSet<usize>,
    /// The modules of the root URLs, which `ROOT_URLCONF` names.
    root_urls: FxHashSet<usize>,
    /// The modules of URLs that `include` names.
    included_urls: FxHashSet<usize>,
}

impl Rules for Django {
    fn new(context: &Context, settings: &Settings) -> Self {
        django_modules(context, settings.django_settings)
    }

    /// Check whether Django or a server reads or calls a definition by itself.
    fn uses(&self, context: &Context, definition: &Definition, module: usize) -> bool {
        let name = definition.name.as_str();
        if let Some(class) = context.class_name(definition, module) {
            if definition.kind == Kind::Class {
                return false;
            }
            let app_config_member = match definition.kind {
                Kind::Attribute => DJANGO_APP_CONFIG_ATTRIBUTES.contains(&name),
                Kind::Method => DJANGO_APP_CONFIG_METHODS.contains(&name),
                _ => false,
            };
            let migration_member =
                definition.kind == Kind::Attribute && DJANGO_MIGRATION_ATTRIBUTES.contains(&name);
            return (app_config_member && context.inherits(module, class, &DJANGO_APP_CONFIGS))
                || (migration_member && context.inherits(module, class, &DJANGO_MIGRATIONS));
        }
        if definition.kind == Kind::Class {
            return definition.owner.is_none()
                && (context.inherits(module, name, &DJANGO_APP_CONFIGS)
                    || (name == DJANGO_MIGRATION_CLASS
                        && context.inherits(module, name, &DJANGO_MIGRATIONS)));
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
            || definition
                .call
                .as_ref()
                .is_some_and(|call| context.resolves_to(module, call, &DJANGO_APPLICATIONS))
    }
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
    }
}

/// Check whether a name is uppercase, as Python's `str.isupper`: it has a cased letter, and no
/// lowercase one.
fn is_uppercase(name: &str) -> bool {
    name.chars().any(char::is_uppercase) && !name.chars().any(char::is_lowercase)
}
