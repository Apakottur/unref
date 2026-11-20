//! SQLAlchemy maps the attributes of its declarative models: the subclasses of
//! `sqlalchemy.orm.DeclarativeBase`, `DeclarativeBaseNoMeta` or `MappedAsDataclass`, of a class
//! that `declarative_base()` makes or that `as_declarative()` decorates, and the classes, such as
//! mixins, that they inherit from. An attribute is mapped when its annotation is `Mapped`, or its
//! value is a call of a construct such as `mapped_column`, `Column` or `relationship`. SQLAlchemy
//! calls the functions that `event.listens_for`, `validates`, `reconstructor` and `declared_attr`
//! register.

use rustc_hash::FxHashSet;

use crate::definitions::{Definition, Kind};

use super::FrameworkRules;

// SQLAlchemy's names are matched anywhere under `sqlalchemy`, by their end from a dot, as it gives most
// of them out from several modules, such as `sqlalchemy.Column` and `sqlalchemy.schema.Column`.
/// The base classes of declarative models.
pub(super) const SQLALCHEMY_BASES: [&str; 3] = [
    ".DeclarativeBase",
    ".DeclarativeBaseNoMeta",
    ".MappedAsDataclass",
];
/// The function that makes a base class of declarative models, and the decorator that makes one.
pub(super) const SQLALCHEMY_BASE_FUNCTIONS: [&str; 1] = [".declarative_base"];
pub(super) const SQLALCHEMY_BASE_DECORATORS: [&str; 1] = [".as_declarative"];
/// The annotations and constructs of mapped attributes.
pub(super) const SQLALCHEMY_ANNOTATIONS: [&str; 3] =
    [".Mapped", ".WriteOnlyMapped", ".DynamicMapped"];
pub(super) const SQLALCHEMY_CONSTRUCTS: [&str; 9] = [
    ".Column",
    ".mapped_column",
    ".relationship",
    ".column_property",
    ".composite",
    ".synonym",
    ".deferred",
    ".query_expression",
    ".dynamic_loader",
];
/// The decorators that register a function with SQLAlchemy.
pub(super) const SQLALCHEMY_DECORATORS: [&str; 6] = [
    ".listens_for",
    ".validates",
    ".reconstructor",
    ".declared_attr",
    ".declared_attr.directive",
    ".declared_attr.cascading",
];

impl FrameworkRules<'_> {
    /// Check whether SQLAlchemy maps or calls a definition by itself.
    pub(super) fn sqlalchemy_uses(&self, definition: &Definition, module: usize) -> bool {
        if definition.kind.is_function() {
            return definition.decorators.iter().any(|decorator| {
                self.resolves_to_sqlalchemy(module, decorator, &SQLALCHEMY_DECORATORS)
            });
        }
        let Some(class) = &definition.class else {
            return false;
        };
        if definition.kind != Kind::Attribute {
            return false;
        }
        self.mapped_classes
            .contains(&self.dotted_name(module, class))
            && (definition.annotation.as_ref().is_some_and(|annotation| {
                self.resolves_to_sqlalchemy(module, annotation, &SQLALCHEMY_ANNOTATIONS)
            }) || definition.call.as_ref().is_some_and(|call| {
                self.resolves_to_sqlalchemy(module, call, &SQLALCHEMY_CONSTRUCTS)
            }))
    }

    /// Get the dotted names of the declarative models of SQLAlchemy, and of the classes that they
    /// inherit from, given the classes of the project and the base classes that it makes.
    pub(super) fn sqlalchemy_classes(
        &self,
        classes: &[(usize, String)],
        made: &FxHashSet<String>,
    ) -> FxHashSet<String> {
        // A variable that a class inherits from in its own module is among its base classes by its name
        // alone.
        let mut models = made.clone();
        for (module, name) in classes {
            let ancestors = self.hierarchy.ancestors(*module, name);
            let local = ancestors
                .iter()
                .filter(|ancestor| !ancestor.contains('.'))
                .map(|ancestor| self.dotted_name(*module, ancestor));
            let inherits_made = ancestors.iter().any(|ancestor| made.contains(ancestor))
                || local.into_iter().any(|ancestor| made.contains(&ancestor));
            if is_sqlalchemy(&ancestors, &SQLALCHEMY_BASES) || inherits_made {
                models.insert(self.dotted_name(*module, name));
            }
        }

        // A model can inherit from a model of another module, whose base class is known only there.
        let mut mapped = models.clone();
        for (module, name) in classes {
            let ancestors = self.hierarchy.ancestors(*module, name);
            let dotted = self.dotted_name(*module, name);
            if models.contains(&dotted)
                || ancestors.iter().any(|ancestor| models.contains(ancestor))
            {
                mapped.insert(dotted);
                mapped.extend(ancestors.iter().cloned());
            }
        }
        mapped
    }

    /// Check whether an expression of a module resolves to a name of SQLAlchemy that ends with one of
    /// the given ends.
    pub(super) fn resolves_to_sqlalchemy(
        &self,
        module: usize,
        expression: &str,
        ends: &[&str],
    ) -> bool {
        is_sqlalchemy(&self.hierarchy.resolve(module, expression), ends)
    }
}

/// Check whether one of the dotted names of something is a name of SQLAlchemy that ends with one of
/// the given ends, such as `sqlalchemy.orm.mapped_column` for `.mapped_column`.
pub(super) fn is_sqlalchemy(names: &FxHashSet<String>, ends: &[&str]) -> bool {
    names
        .iter()
        .any(|name| name.starts_with("sqlalchemy.") && ends.iter().any(|end| name.ends_with(end)))
}
