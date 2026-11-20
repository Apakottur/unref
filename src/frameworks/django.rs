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

use crate::classes::ClassHierarchy;
use crate::definitions::{Definition, Kind};
use crate::modules::{ModuleName, resolve};

use super::{FrameworkRules, ProjectFile, is_top_level};

/// The environment variable that names the settings module of Django, and `os.environ.setdefault`,
/// which `startproject` sets it with.
pub(super) const DJANGO_SETTINGS_VARIABLE: &str = "DJANGO_SETTINGS_MODULE";
pub(super) const OS_ENVIRON_SETDEFAULT: [&str; 1] = ["os.environ.setdefault"];
/// The setting that names the module of the root URLs.
pub(super) const DJANGO_ROOT_URLCONF: &str = "ROOT_URLCONF";
/// `include`, which names a module of URLs, as `django.urls` gives it out, where it is defined, and as
/// `django.conf.urls` gave it out before Django 4.
pub(super) const DJANGO_INCLUDES: [&str; 3] = [
    "django.urls.include",
    "django.urls.conf.include",
    "django.conf.urls.include",
];
/// The variables that Django reads from a module of URLs, and those that it reads from the module of
/// the root URLs only, or from an included one only.
pub(super) const DJANGO_URLS_VARIABLES: [&str; 1] = ["urlpatterns"];
pub(super) const DJANGO_ROOT_URLS_VARIABLES: [&str; 4] =
    ["handler400", "handler403", "handler404", "handler500"];
pub(super) const DJANGO_INCLUDED_URLS_VARIABLES: [&str; 1] = ["app_name"];
/// The functions that make the application that a server loads.
pub(super) const DJANGO_APPLICATIONS: [&str; 2] = [
    "django.core.wsgi.get_wsgi_application",
    "django.core.asgi.get_asgi_application",
];
/// `AppConfig`, as `django.apps` gives it out, and where it is defined, with the attributes and
/// methods of its subclasses that Django reads and calls.
pub(super) const DJANGO_APP_CONFIGS: [&str; 2] =
    ["django.apps.AppConfig", "django.apps.config.AppConfig"];
pub(super) const DJANGO_APP_CONFIG_ATTRIBUTES: [&str; 6] = [
    "name",
    "label",
    "verbose_name",
    "path",
    "default",
    "default_auto_field",
];
pub(super) const DJANGO_APP_CONFIG_METHODS: [&str; 1] = ["ready"];
/// `Migration`, as `django.db.migrations` gives it out, and where it is defined, with the attributes
/// of its subclasses that Django reads.
pub(super) const DJANGO_MIGRATIONS: [&str; 2] = [
    "django.db.migrations.Migration",
    "django.db.migrations.migration.Migration",
];
/// The name of the class that Django loads from a migration module.
pub(super) const DJANGO_MIGRATION_CLASS: &str = "Migration";
pub(super) const DJANGO_MIGRATION_ATTRIBUTES: [&str; 6] = [
    "operations",
    "dependencies",
    "run_before",
    "replaces",
    "initial",
    "atomic",
];

/// The modules that Django reads by their names, by their indices.
#[derive(Default)]
pub(super) struct DjangoModules {
    /// The settings modules.
    pub(super) settings: FxHashSet<usize>,
    /// The modules of the root URLs, which `ROOT_URLCONF` names.
    pub(super) root_urls: FxHashSet<usize>,
    /// The modules of URLs that `include` names.
    pub(super) included_urls: FxHashSet<usize>,
}

impl FrameworkRules<'_> {
    /// Check whether Django or a server reads or calls a definition by itself.
    pub(super) fn django_uses(&self, definition: &Definition, module: usize) -> bool {
        let name = definition.name.as_str();
        if let Some(class) = &definition.class {
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
            return (app_config_member && self.inherits(module, class, &DJANGO_APP_CONFIGS))
                || (migration_member && self.inherits(module, class, &DJANGO_MIGRATIONS));
        }
        if definition.kind == Kind::Class {
            return definition.owner.is_none()
                && (self.inherits(module, name, &DJANGO_APP_CONFIGS)
                    || (name == DJANGO_MIGRATION_CLASS
                        && self.inherits(module, name, &DJANGO_MIGRATIONS)));
        }
        if definition.kind != Kind::Variable || definition.owner.is_some() {
            return false;
        }
        let django = &self.django;
        (django.settings.contains(&module) && is_uppercase(name))
            || ((django.root_urls.contains(&module) || django.included_urls.contains(&module))
                && DJANGO_URLS_VARIABLES.contains(&name))
            || (django.root_urls.contains(&module) && DJANGO_ROOT_URLS_VARIABLES.contains(&name))
            || (django.included_urls.contains(&module)
                && DJANGO_INCLUDED_URLS_VARIABLES.contains(&name))
            || definition
                .call
                .as_ref()
                .is_some_and(|call| self.resolves_to(module, call, &DJANGO_APPLICATIONS))
    }
}

/// Find the modules that Django reads by their names, given the settings modules that are given by
/// their resolved paths.
pub(super) fn django_modules(
    files: &[ProjectFile],
    module_names: &[ModuleName],
    hierarchy: &ClassHierarchy,
    given: &FxHashSet<PathBuf>,
) -> DjangoModules {
    let mut by_name: FxHashMap<&str, Vec<usize>> = FxHashMap::default();
    for (module, name) in module_names.iter().enumerate() {
        by_name.entry(name.name.as_str()).or_default().push(module);
    }
    let modules = |name: &str| by_name.get(name).into_iter().flatten().copied();

    let mut pending: Vec<usize> = Vec::new();
    let mut included_urls = FxHashSet::default();
    for (module, (path, _, references)) in files.iter().enumerate() {
        if !given.is_empty() && given.contains(&resolve(path)) {
            pending.push(module);
        }
        for (called, arguments) in &references.string_calls {
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
        for import in &files[module].2.imports {
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
        for definition in files[module].1 {
            if definition.kind == Kind::Variable
                && is_top_level(definition)
                && definition.name == DJANGO_ROOT_URLCONF
                && let Some(urls) = &definition.string
            {
                root_urls.extend(modules(urls));
            }
        }
    }
    DjangoModules {
        settings,
        root_urls,
        included_urls,
    }
}

/// Check whether a name is uppercase, as Python's `str.isupper`: it has a cased letter, and no
/// lowercase one.
pub(super) fn is_uppercase(name: &str) -> bool {
    name.chars().any(char::is_uppercase) && !name.chars().any(char::is_lowercase)
}
