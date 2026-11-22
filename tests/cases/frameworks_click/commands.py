import rich_click as click

remote = click.RichGroup("remote")


# rich-click gives out Click's API.
@click.group()
def styled():
    return None


@styled.command()
def styled_command():
    return None


styled
