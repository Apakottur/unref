from django.apps import AppConfig


# Django makes the subclasses of `AppConfig`, reads their attributes and calls `ready`.
class PollsConfig(AppConfig):
    name = "polls"
    verbose_name = "Polls"
    default_auto_field = "django.db.models.BigAutoField"

    def ready(self):
        pass

    def unused_method(self):  # expect: 9 [unused-method]
        pass

    unused_attribute = 1  # expect: 5 [unused-attribute]


class OtherConfig:  # expect: 7 [unused-class]
    name = "other"  # expect: 5 [unused-attribute] transitive
