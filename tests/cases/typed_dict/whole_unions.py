from unions import Alert, Failure, KeyedAlias, Message, Signal


# A value annotated with a union alias of TypedDicts, imported from another module, reads each of them
# whole.
def show(
    message: Message, failure: Failure, alert: Alert, signal: Signal, keyed: KeyedAlias
) -> None:
    {**message}, dict(failure), list(alert), {**signal}, keyed["keyed"]


show({}, None, None, {}, None)
