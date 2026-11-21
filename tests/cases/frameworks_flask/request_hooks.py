import flask
from flask import after_this_request, copy_current_request_context, stream_with_context
from flask.signals import template_rendered

from app import app


# Flask calls a function that `after_this_request` registers, after the current request.
@app.route("/profile")
def profile():
    @after_this_request
    def add_header(response):
        return response

    @flask.after_this_request
    def set_cookie(response):
        return response

    return ""


# These only wrap a function, which still needs a reference.
@app.route("/stream")
def stream():
    @copy_current_request_context
    def send_mail():  # expect: 9 [unused-function]
        return None

    @stream_with_context
    def generate():  # expect: 9 [unused-function]
        yield ""

    return ""


# Flask calls the receivers that the decorators of its signals connect.
@flask.request_started.connect
def log_request(sender, **extra):
    return None


@template_rendered.connect_via(app)
def log_template(sender, template, context, **extra):
    return None


# Not a signal of Flask.
class Signal:
    def connect(self, receiver):
        return receiver


custom = Signal()


@custom.connect
def not_a_receiver(sender):  # expect: 5 [unused-function]
    return None


# Not Flask's `after_this_request`.
def after_this_request_of_mine(function):
    return function


@app.route("/other")
def other():
    @after_this_request_of_mine
    def not_registered(response):  # expect: 9 [unused-function]
        return response

    return ""
