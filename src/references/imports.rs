//! The names that the imports of a module bind.

use ruff_python_ast as ast;
use rustc_hash::FxHashMap;

use super::{Binding, Import, Walker};

impl Walker<'_, '_> {
    /// Bind the names that an `import` statement imports.
    pub(super) fn visit_import(&mut self, import: &ast::StmtImport) {
        let bindings = import.names.iter().map(|alias| match &alias.asname {
            // `import package.module` binds `package`.
            None => {
                let package = alias.name.split('.').next().unwrap_or_default().to_string();
                (
                    package.clone(),
                    Binding {
                        module: Some(package),
                        level: 0,
                        name: None,
                    },
                )
            }
            Some(asname) => (
                asname.to_string(),
                Binding {
                    module: Some(alias.name.to_string()),
                    level: 0,
                    name: None,
                },
            ),
        });
        let bindings = bindings.collect();
        self.bind_imports(bindings);
    }

    /// Add the names that a `from` import imports, and bind them.
    pub(super) fn visit_import_from(&mut self, import: &ast::StmtImportFrom) {
        let module = import.module.as_ref().map(ToString::to_string);
        let mut bindings = FxHashMap::default();
        for alias in &import.names {
            self.references.imports.insert(Import {
                owner: self.owner,
                module: module.clone(),
                level: import.level,
                name: alias.name.to_string(),
            });
            let bound = alias.asname.as_ref().unwrap_or(&alias.name).to_string();
            bindings.insert(
                bound,
                Binding {
                    module: module.clone(),
                    level: import.level,
                    name: Some(alias.name.to_string()),
                },
            );
        }
        self.bind_imports(bindings);
    }
}
