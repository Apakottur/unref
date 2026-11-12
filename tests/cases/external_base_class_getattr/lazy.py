def __getattr__(name):
    if name == "Handler":
        from logging import Handler

        return Handler
    if name == "Encoder":
        import json

        return json.JSONEncoder
    if name == "TooFar":
        # More dots than there are packages.
        from .... import TooFar

        return TooFar
    raise AttributeError(name)
