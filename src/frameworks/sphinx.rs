//! Sphinx runs its configuration file, `conf.py`, and reads the settings it sets at the top level,
//! its own and those of its extensions, which are open-ended, such as `myst_enable_extensions`. It
//! calls the `setup` function of the file as that of an extension, and `sphinx.ext.linkcode` calls
//! its `linkcode_resolve` function. Its configuration files are those that the `configuration`
//! option of the `frameworks.sphinx` table names, or else each `conf.py` that sets `project`,
//! `extensions` or `html_theme` at the top level, as `sphinx-quickstart` writes. No setting starts
//! with an underscore, so such a name is left to be reported.

use std::path::{Path, PathBuf};

use rustc_hash::FxHashSet;

use crate::definitions::{Definition, Kind};
use crate::modules::resolve;

use super::{Context, Rules, Settings, file_name, is_top_level};

/// The name of Sphinx's configuration file, which it reads from its configuration directory.
const SPHINX_CONF: &str = "conf.py";
/// The settings that make a `conf.py` that of Sphinx when it sets one at the top level.
const SPHINX_MARKERS: [&str; 3] = ["project", "extensions", "html_theme"];
/// The functions of the configuration file that Sphinx and its extensions call.
const SPHINX_FUNCTIONS: [&str; 2] = ["setup", "linkcode_resolve"];

/// The rules of Sphinx.
pub(super) struct Sphinx {
    /// The configuration files that are given, as resolved paths, or None to find them by their
    /// contents.
    given: Option<FxHashSet<PathBuf>>,
    /// The configuration files, by their indices.
    configs: FxHashSet<usize>,
}

impl Rules for Sphinx {
    fn new(_context: &Context, settings: &Settings) -> Self {
        Self {
            given: settings.sphinx_configuration.cloned(),
            configs: FxHashSet::default(),
        }
    }

    fn file(&mut self, context: &Context, module: usize) {
        let file = &context.files[module];
        let is_config = match &self.given {
            Some(configuration) => configuration.contains(&resolve(file.path)),
            None => is_sphinx_config(file.path, file.definitions),
        };
        if is_config {
            self.configs.insert(module);
        }
    }

    /// Check whether Sphinx reads or calls a definition of its configuration file by itself.
    fn uses(&self, _context: &Context, definition: &Definition, module: usize) -> bool {
        let name = definition.name.as_str();
        self.configs.contains(&module)
            && is_top_level(definition)
            && ((definition.kind == Kind::Variable && !name.starts_with('_'))
                || (definition.kind == Kind::Function && SPHINX_FUNCTIONS.contains(&name)))
    }
}

/// Check whether a file is the configuration file of Sphinx: a `conf.py` that sets one of Sphinx's
/// marker settings at the top level.
fn is_sphinx_config(path: &Path, definitions: &[Definition]) -> bool {
    file_name(path) == SPHINX_CONF
        && definitions.iter().any(|definition| {
            definition.kind == Kind::Variable
                && is_top_level(definition)
                && SPHINX_MARKERS.contains(&definition.name.as_str())
        })
}
