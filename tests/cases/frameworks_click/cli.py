import click
from click import group as click_group

import commands


@click.group()
@click.option("--verbose", is_flag=True)
@click.pass_context
def cli(context, verbose):
    context.obj = verbose


# Click runs the commands and groups that the decorators of a group register with it.
@cli.command()
@click.option("--name")
def hello(name):
    return name


@cli.command
def without_parentheses():
    return None


@cli.group("db")
def database():
    return None


@database.command("migrate")
@click.pass_obj
def migrate_database(verbose):
    return verbose


@cli.result_callback()
def process_result(result, verbose):
    return result


# A group made with an alias of Click's decorator.
@click_group()
def tools():
    return None


@tools.command()
def lint():
    return None


# A group of commands made with `Group`, or with a subclass of it.
admin = click.Group("admin")


@admin.command()
def create_user():
    return None


class OrderedGroup(click.Group):
    pass


ordered = OrderedGroup()


@ordered.command()
def first():
    return None


def register(parent):
    local = click.Group("local")

    @local.command()
    def local_command():
        return None

    parent.add_command(local)


# A command of a group of another module.
@commands.remote.command()
def remote_command():
    return None


# Click's own decorators make a command that is registered nowhere: it runs only if something refers to
# it, as `add_command` does.
@click.command()
def added():
    return None


@click.command()
def standalone():  # expect: 5 [unused-function]
    return None


# Not a group: a decorator with the same name is not a command.
class Registry:
    def command(self):
        return lambda function: function


registry = Registry()


@registry.command()
def not_a_command():  # expect: 5 [unused-function]
    return None


def make_command(parent):
    @parent.command()
    def unknown_parent():  # expect: 9 [unused-function]
        return None


cli.add_command(added)
cli.add_command(admin)
cli.add_command(ordered)
cli.add_command(tools)
cli.add_command(commands.remote)

if __name__ == "__main__":
    cli()

register, make_command
