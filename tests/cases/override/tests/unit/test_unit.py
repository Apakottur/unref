# Every override that covers a file applies to it.
class TestInUnit:
    def test_method(self):
        return 1

    def helper(self):  # expect: 9 [unused-method]
        return 2
