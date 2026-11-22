from django.core.management import BaseCommand


# Not in a package of commands.
class Command(BaseCommand):  # expect: 7 [unused-class]
    pass
