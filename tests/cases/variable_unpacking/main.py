first, second = 1, 2  # expect: unused variable, unused variable
[third, fourth] = 3, 4  # expect: unused variable, unused variable
head, *rest = [5, 6, 7]  # expect: unused variable, unused variable
(nested, (deep, deeper)) = 8, (9, 10)  # expect: unused variable, unused variable, unused variable
chained = also_chained = 11  # expect: unused variable, unused variable
