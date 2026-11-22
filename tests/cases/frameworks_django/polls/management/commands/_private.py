from django.core.management import BaseCommand


# A module whose name starts with `_` is not a command.
class Command(BaseCommand):  # expect: 7 [unused-class]
    def handle(self, *args, **options):
        pass
