def __getattr__(name):
    if name == "Handler":
        from logging import Handler

        return Handler
    if name == "Encoder":
        import json

        return json.JSONEncoder
    raise AttributeError(name)
