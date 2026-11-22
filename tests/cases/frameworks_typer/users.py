import typer

app = typer.Typer()


@app.command("create")
def create_user(name: str):
    return name
