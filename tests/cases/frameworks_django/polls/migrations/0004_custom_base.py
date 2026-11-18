from polls.migration_base import BaseMigration


# A subclass of a project's own base class of migrations.
class Migration(BaseMigration):
    dependencies = [("polls", "0003_defined")]
