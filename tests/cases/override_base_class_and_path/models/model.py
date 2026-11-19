from pydantic import BaseModel


class Model(BaseModel):
    table_name = {}


class Plain:
    table_name = {}  # expect: 5 [unused-attribute]


Model, Plain
