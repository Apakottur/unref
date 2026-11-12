import typer
import editable_module
from envonly.types import Text, TypeDecorator


# A base class is found in the site-packages of the environment of the project, which the check does not
# import from.
class Upper(TypeDecorator):
    impl = Text
    cache_ok = True

    def process_bind_param(self, value, dialect):
        return value.upper()

    def own_method(self):  # expect: unused method
        return self


# A base class is found in a directory that a `.pth` file of the environment adds.
class FromEditable(editable_module.Base):
    def hook(self):
        return self


# The environment of the project replaces the site-packages of the check: `typer` is installed with the
# check, but not in the environment of the project.
class App(typer.Typer):
    def command(self):  # expect: unused method
        return self


print(Upper, FromEditable, App)
