import flask

from config import Production, Unread

TEST_KEY = "key"
test_value = "value"  # expect: 1 [unused-variable]

app = flask.Flask(__name__)
app.config.from_object(__name__)
app.config.from_object("settings")
app.config.from_object(Production)


def build():
    local = flask.Flask(__name__)
    local.config.from_object("config.Development")
    return local


build()
other = flask.Config(".")
other.from_object(Unread)
