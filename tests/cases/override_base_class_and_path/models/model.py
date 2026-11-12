from pydantic import BaseModel


class Model(BaseModel):
    table_name = {}


class Plain:
    table_name = {}  # expect: unused attribute


print(Model, Plain)
