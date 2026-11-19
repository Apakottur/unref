import click
from flask import Blueprint, Flask
from flask.cli import AppGroup

app = Flask(__name__)
bp = Blueprint("cli", __name__)
users = AppGroup("users")


# Flask runs the commands of the CLI of an application or a blueprint, and of its groups.
@app.cli.command()
def init_db():
    return None


@bp.cli.group()
def translate():
    return None


@translate.command()
def compile_messages():
    return None


@translate.group()
def catalog():
    return None


@catalog.command()
@click.argument("name")
def update_catalog(name):
    return None


# A group of commands made with Flask's `AppGroup`.
@users.command("create")
def create_user():
    return None


def register(local):
    group = AppGroup("local")

    @group.command()
    def local_command():
        return None

    local.cli.add_command(group)


# Not Flask's CLI: a decorator with the same name is not a command.
class Tool:
    def __init__(self):
        self.cli = click.Group()


tool = Tool()


@tool.cli.command()
def not_a_command():  # expect: 5 [unused-function]
    return None


@click.group()
def plain():
    return None


@plain.command()
def plain_command():  # expect: 5 [unused-function]
    return None


app.cli.add_command(users)
app.register_blueprint(bp)
register, plain
