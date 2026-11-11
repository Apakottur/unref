from pydantic import BaseModel


class Model(BaseModel):
    model_config = {}


class Plain:
    model_config = {}  # expect: unused attribute


print(Model, Plain)
