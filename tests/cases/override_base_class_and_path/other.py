from pydantic import BaseModel


class Model(BaseModel):
    model_config = {}  # expect: unused attribute


print(Model)
