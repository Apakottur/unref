from typing import TypedDict


class Counts(TypedDict):
    augmented_total: int
    assigned_total: int  # expect: 5 [unused-attribute]


counts = Counts(augmented_total=0, assigned_total=0)
counts["augmented_total"] += 1
counts["assigned_total"] = 2
