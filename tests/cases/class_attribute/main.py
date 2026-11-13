class Class:  # expect: 7 [unused-class]
    plain = 1  # expect: 5 [unused-attribute] transitive
    annotated: int = 2  # expect: 5 [unused-attribute] transitive
    declared: int  # expect: 5 [unused-attribute] transitive
    first, second = 3, 4  # expect: 5 [unused-attribute] transitive, 12 [unused-attribute] transitive
