# A framework finds an instance that is imported by its name from the project root.
from top.inner.web import app


@app.get("/")
def index():
    return {}
