from pydantic import BaseModel


class Model(BaseModel):
    table_name = {}


class Plain:
    table_name = {}  # expect: 5 [unused-attribute]


print(Model, Plain)
