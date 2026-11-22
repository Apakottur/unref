import typer
from typer import Typer

import users

app = typer.Typer()
app.add_typer(users.app, name="users")


# Typer runs the functions that the decorators of an application register.
@app.callback()
def main(verbose: bool = False):
    return verbose


@app.command()
def hello(name: str):
    return name


@app.command("bye")
def goodbye(name: str):
    return name


class CustomTyper(Typer):
    pass


custom = CustomTyper()


@custom.command()
def custom_command():
    return None


def build():
    local = typer.Typer()

    @local.command()
    def local_command():
        return None

    return local


# `typer.run` refers to the function it runs.
def single(name: str):
    return name


def run_single():
    typer.run(single)


# Not a Typer application: a decorator with the same name is not a command.
class Registry:
    def command(self):
        return lambda function: function

    def callback(self):
        return lambda function: function


registry = Registry()


@registry.command()
def not_a_command():  # expect: 5 [unused-function]
    return None


@registry.callback()
def not_a_callback():  # expect: 5 [unused-function]
    return None


def unused():  # expect: 5 [unused-function]
    return None


build, run_single, custom

if __name__ == "__main__":
    app()
