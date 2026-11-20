# A string in a quoted annotation is read as in one that is not quoted: a `Literal` key reads it.
from typing import Literal, TypedDict


class Row(TypedDict):
    name: str
    other: str  # expect: 5 [unused-attribute]


def get(row: Row, key: Literal["name"]):
    return row


get(Row(name="a", other="b"), "name")


class QuotedRow(TypedDict):
    label: str
    unread: str  # expect: 5 [unused-attribute]


def get_quoted(row: QuotedRow, key: "Literal['label']"):
    return row


get_quoted(QuotedRow(label="a", unread="b"), "label")
