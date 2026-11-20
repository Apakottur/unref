//! Sphinx runs its configuration file, `conf.py`, and reads the settings it sets at the top level,
//! its own and those of its extensions, which are open-ended, such as `myst_enable_extensions`. It
//! calls the `setup` function of the file as that of an extension, and `sphinx.ext.linkcode` calls
//! its `linkcode_resolve` function. Its configuration files are those that the `configuration`
//! option of the `frameworks.sphinx` table names, or else each `conf.py` that sets `project`,
//! `extensions` or `html_theme` at the top level, as `sphinx-quickstart` writes. No setting starts
//! with an underscore, so such a name is left to be reported.

use std::path::Path;

use crate::definitions::{Definition, Kind};

use super::{FrameworkRules, file_name, is_top_level};

/// The name of Sphinx's configuration file, which it reads from its configuration directory.
pub(super) const SPHINX_CONF: &str = "conf.py";
/// The settings that make a `conf.py` that of Sphinx when it sets one at the top level.
pub(super) const SPHINX_MARKERS: [&str; 3] = ["project", "extensions", "html_theme"];
/// The functions of the configuration file that Sphinx and its extensions call.
pub(super) const SPHINX_FUNCTIONS: [&str; 2] = ["setup", "linkcode_resolve"];

impl FrameworkRules<'_> {
    /// Check whether Sphinx reads or calls a definition of its configuration file by itself.
    pub(super) fn sphinx_uses(&self, definition: &Definition, module: usize) -> bool {
        let name = definition.name.as_str();
        self.sphinx_configs.contains(&module)
            && is_top_level(definition)
            && ((definition.kind == Kind::Variable && !name.starts_with('_'))
                || (definition.kind == Kind::Function && SPHINX_FUNCTIONS.contains(&name)))
    }
}

/// Check whether a file is the configuration file of Sphinx: a `conf.py` that sets one of Sphinx's
/// marker settings at the top level.
pub(super) fn is_sphinx_config(path: &Path, definitions: &[Definition]) -> bool {
    file_name(path) == SPHINX_CONF
        && definitions.iter().any(|definition| {
            definition.kind == Kind::Variable
                && is_top_level(definition)
                && SPHINX_MARKERS.contains(&definition.name.as_str())
        })
}
