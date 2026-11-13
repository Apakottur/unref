from dataclasses import dataclass, field
from pathlib import Path

from unref._definitions import Definition
from unref._modules import ModuleName
from unref._references import Binding, References


@dataclass(frozen=True)
class ModuleSymbols:
    """
    What a module defines and imports at its top level, as the base classes and decorators of its
    classes are followed through it, whether it is a module of the project or one outside it.

    Attributes:
        path: The file of the module.
        name: The name that the module is imported by.
        definitions: What the module defines.
        bindings: The names that the module imports at its top level.
        stars: The modules that the module star imports at its top level, each as it is written and
            with its number of leading dots.
        lazy: The names that the module gives out from its `__getattr__`, each with the imports of it.
        bases: The classes and functions of the module, by their name in the module (`Outer.Inner` for
            a nested class), with their base classes as they are written. A function has none.
        decorators: The classes of the module, by their name in the module, with their decorators as
            they are written.
        members: The names of the members of the classes of the module, by the name of the class.
    """

    path: Path
    name: ModuleName
    definitions: list[Definition]
    bindings: dict[str, Binding]
    stars: list[tuple[str | None, int]]
    lazy: dict[str, list[Binding]]
    bases: dict[str, tuple[str, ...]] = field(default_factory=dict)
    decorators: dict[str, tuple[str, ...]] = field(default_factory=dict)
    members: dict[str, set[str]] = field(default_factory=dict)

    @classmethod
    def of(
        cls,
        path: Path,
        name: ModuleName,
        definitions: list[Definition],
        references: References,
        lazy: dict[str, list[Binding]],
    ) -> "ModuleSymbols":
        """
        Collect what a module defines and imports at its top level.

        Args:
            path: The file of the module.
            name: The name that the module is imported by.
            definitions: What the module defines.
            references: What the module refers to.
            lazy: The names that the module gives out from its `__getattr__`, each with the imports of it.

        Returns:
            The symbols of the module.
        """
        symbols = cls(path, name, definitions, references.bindings, _star_imports(references), lazy)
        for definition in definitions:
            if definition.kind in ("class", "function"):
                symbols.bases[definition.qualified_name] = definition.bases
            if definition.kind == "class":
                symbols.decorators[definition.qualified_name] = definition.decorators
            if definition.owner is not None:
                symbols.members.setdefault(definition.owner, set()).add(definition.name)
        return symbols


def _star_imports(references: References) -> list[tuple[str | None, int]]:
    """
    Get the modules that a module star imports at its top level.

    Args:
        references: What the module refers to.

    Returns:
        The modules, each as it is written and with its number of leading dots.
    """
    return sorted(
        ((source, level) for owner, source, level, name in references.imports if owner is None and name == "*"),
        key=str,
    )
