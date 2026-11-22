from django.core.management.base import BaseCommand


# `manage.py` runs the class named `Command` of a module of commands, and calls its `handle`.
class Command(BaseCommand):
    help = "Closes a poll"

    def add_arguments(self, parser):
        parser.add_argument("poll_id")

    def handle(self, *args, **options):
        pass

    def unused_method(self):  # expect: 9 [unused-method]
        pass


# Only the class named `Command` is run.
class Other(BaseCommand):  # expect: 7 [unused-class]
    pass
