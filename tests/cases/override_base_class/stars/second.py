from pydantic import BaseModel

from stars import *


class Starred(BaseModel):
    table_name = {}
