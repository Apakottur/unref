from django.db.migrations.migration import Migration as DefinedMigration


# `Migration` where it is defined.
class Migration(DefinedMigration):
    dependencies = [("polls", "0001_initial")]
