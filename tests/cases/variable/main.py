PLAIN = 1  # expect: 1 [unused-variable]
ANNOTATED: int = 2  # expect: 1 [unused-variable]
DECLARED: int  # expect: 1 [unused-variable]
