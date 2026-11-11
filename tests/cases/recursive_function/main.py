# A function that only calls itself counts as referenced.
def countdown(value):
    if value:
        countdown(value - 1)
