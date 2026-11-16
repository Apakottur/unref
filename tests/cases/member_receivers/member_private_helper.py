# Two classes have a private helper of the same name, and only one of them is used.
class Parser:
    def parse_text(self, text):
        return self._split(text)

    def _split(self, text):
        return text.split()


class Formatter:
    def format_words(self, words):
        return " ".join(words)

    def _split(self, text):  # expect: 9 [unused-method]
        return text.splitlines()


print(Parser().parse_text("a b"))
print(Formatter().format_words(["a"]))
