from django.db import migrations


# A base class of migrations, which the class named `Migration` of a migration inherits from.
class BaseMigration(migrations.Migration):
    atomic = False

    unused_attribute = 1  # expect: 5 [unused-attribute]
