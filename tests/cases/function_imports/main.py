import pytest


# A base class that a function imports is followed as one that the module imports.
def make_handler():
    import logging

    class LogHandler(logging.Handler):
        def emit(self, record):
            return record

        def own_method(self):  # expect: 13 [unused-method]
            return self

    return LogHandler


# So is one imported by a function that the class is nested deeper in.
def make_nested_handler():
    from logging import Handler

    def inner():
        class NestedHandler(Handler):
            def emit(self, record):
                return record

        return NestedHandler

    return inner


# A name that another function imports is not one of this function.
def import_handler():
    from logging import Handler

    return Handler


def make_unrelated():
    class Unrelated(Handler):  # noqa: F821
        def emit(self, record):  # expect: 13 [unused-method]
            return record

    return Unrelated


# The rules of frameworks follow the imports of functions.
@pytest.fixture
def client():
    from fastapi import FastAPI
    from pydantic import BaseModel, field_validator

    app = FastAPI()

    class Item(BaseModel):
        name: str

        @field_validator("name")
        @classmethod
        def check_name(cls, name):
            return name

    @app.get("/items")
    def list_items() -> list[Item]:
        return []

    return app


make_handler(), make_nested_handler(), import_handler(), make_unrelated()
