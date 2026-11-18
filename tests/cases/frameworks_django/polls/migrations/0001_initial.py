from django.db import migrations


def forwards(apps, schema_editor):
    pass


# Django's migration commands load the subclasses of `Migration` and read their attributes.
class Migration(migrations.Migration):
    initial = True
    atomic = False
    dependencies = []
    run_before = []
    replaces = []
    operations = [migrations.RunPython(forwards)]

    unused_attribute = 1  # expect: 5 [unused-attribute]


class NotMigration:  # expect: 7 [unused-class]
    operations = []  # expect: 5 [unused-attribute] transitive
