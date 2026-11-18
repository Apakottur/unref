from django.db.migrations import Migration as ImportedMigration


# `Migration` as `django.db.migrations` gives it out.
class Migration(ImportedMigration):
    dependencies = [("polls", "0001_initial")]
