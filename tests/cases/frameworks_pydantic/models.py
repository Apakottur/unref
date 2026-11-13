import typing
from typing import ClassVar

import pydantic
import pydantic.v1
from pydantic import BaseModel, computed_field, field_serializer, field_validator, model_serializer, model_validator
from pydantic.main import BaseModel as MainBaseModel
from pydantic_settings import BaseSettings


# Pydantic reads the fields of a model and its configuration.
class User(BaseModel):
    model_config = {"frozen": True}

    name: str
    age: int = 0
    nickname: "str | None" = None
    # A quoted annotation that does not parse is still an annotation.
    broken: "str |" = ""

    # Not fields: a class variable, a private attribute and a name without an annotation.
    kind: ClassVar[str] = "user"  # expect: 5 [unused-attribute]
    limit: typing.ClassVar[int] = 1  # expect: 5 [unused-attribute]
    _secret: int = 0  # expect: 5 [unused-attribute]
    plain = 1  # expect: 5 [unused-attribute]

    # Pydantic calls the methods that its decorators register.
    @field_validator("name")
    @classmethod
    def check_name(cls, value):
        return value

    @model_validator(mode="after")
    def check_model(self):
        return self

    @field_serializer("age")
    def dump_age(self, value):
        return value

    @model_serializer
    def dump(self):
        return self

    @computed_field
    @property
    def label(self):
        return self.name

    # Other methods are still reported.
    def helper(self):  # expect: 9 [unused-method]
        return self


# Through the module, and where `BaseModel` is defined.
class FromModule(pydantic.BaseModel):
    name: str

    @pydantic.validator("name")
    def check_name(cls, value):
        return value

    @pydantic.root_validator
    def check_all(cls, values):
        return values


class FromMain(MainBaseModel):
    name: str


# Through a subclass of the project.
class Admin(User):
    level: int


# Through a subclass outside the project, which imports `BaseModel` from `pydantic.main`.
class Settings(BaseSettings):
    model_config = {"env_prefix": "APP_"}
    debug: bool = False


# Pydantic 1, with its configuration class.
class Legacy(pydantic.v1.BaseModel):
    name: str

    class Config:
        orm_mode = True


# Not a model.
class Plain:
    model_config = {}  # expect: 5 [unused-attribute]
    title: str = ""  # expect: 5 [unused-attribute]

    class Config:  # expect: 11 [unused-class]
        orm_mode = True  # expect: 9 [unused-attribute] transitive


print(User, FromModule, FromMain, Admin, Settings, Legacy, Plain)
