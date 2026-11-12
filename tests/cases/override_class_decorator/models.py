import dataclasses
import dataclasses as renamed_module
from dataclasses import dataclass
from dataclasses import dataclass as renamed

import attrs
import decorators
from decorators import record


@dataclass
class Direct:
    field: int


# The arguments of a decorator are ignored.
@dataclass(frozen=True)
class WithArguments:
    field: int


@dataclasses.dataclass
class ThroughModule:
    field: int


@renamed
class ThroughRenamedImport:
    field: int


@renamed_module.dataclass
class ThroughRenamedModule:
    field: int


# A decorator of the project is followed through its imports.
@record
class FromImport:
    field: int


@decorators.record
class FromModule:
    field: int


# The override applies to everything in the class: attributes, methods and what is on the instance.
@dataclass
class Everything:
    field: int

    def method(self):
        self.on_instance = 1


# The override does not apply to a subclass, which is not a data class itself.
class Child(Direct):
    field: int  # expect: unused attribute


# Not a decorator of the override.
@attrs.define
class OtherDecorator:
    field: int  # expect: unused attribute


class Plain:
    field: int  # expect: unused attribute


# A nested class has its own decorators.
@dataclass
class Outer:
    class Inner:
        field: int  # expect: unused attribute


# The override does not apply outside the class.
field: int = 1  # expect: unused variable

print(
    Direct,
    WithArguments,
    ThroughModule,
    ThroughRenamedImport,
    ThroughRenamedModule,
    FromImport,
    FromModule,
    Everything,
    Child,
    OtherDecorator,
    Plain,
    Outer.Inner,
)
