from pydantic import BaseModel


class Base(BaseModel):
    table_name = {}
