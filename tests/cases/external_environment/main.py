import flask
import editable_module
from project_only.types import Text, TypeDecorator


# A base class is found in the site-packages of the environment of the project, which the check does not
# import from.
class Upper(TypeDecorator):
    impl = Text
    cache_ok = True

    def process_bind_param(self, value, dialect):
        return value.upper()

    def own_method(self):  # expect: 9 [unused-method]
        return self


# A base class is found in a directory that a `.pth` file of the environment adds.
class FromEditable(editable_module.Base):
    def hook(self):
        return self


# Installed packages are read from the environment of the project only: `flask` is not installed in
# it.
class App(flask.Flask):
    def command(self):  # expect: 9 [unused-method]
        return self


print(Upper, FromEditable, App)
