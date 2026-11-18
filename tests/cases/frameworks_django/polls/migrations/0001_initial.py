from django.db import migrations


def forwards(apps, schema_editor):
    pass


# Django's migration commands load the class named `Migration` and read its attributes.
class Migration(migrations.Migration):
    initial = True
    atomic = False
    dependencies = []
    run_before = []
    replaces = []
    operations = [migrations.RunPython(forwards)]

    unused_attribute = 1  # expect: 5 [unused-attribute]


# Django loads only the class named `Migration`.
class OtherMigration(migrations.Migration):  # expect: 7 [unused-class]
    operations = []


class NotMigration:  # expect: 7 [unused-class]
    operations = []  # expect: 5 [unused-attribute] transitive
