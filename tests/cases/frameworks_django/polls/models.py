from django.db import models
from django.db.models import ForeignKey


class PollQuerySet(models.QuerySet):
    def published(self):  # expect: 9 [unused-method]
        return self


# Django calls `get_queryset` of a manager for each query.
class PollManager(models.Manager):
    def get_queryset(self):
        return super().get_queryset()

    def unused_method(self):  # expect: 9 [unused-method]
        return None


class Choice(models.Model):
    text = models.CharField(max_length=10)


# The ORM, migrations, admin and forms read the fields of a model, and Django uses its first manager as
# its default manager.
class Poll(models.Model):
    title = models.CharField(max_length=10)
    choice = ForeignKey(Choice, on_delete=models.CASCADE)
    objects = PollManager()
    # Another manager is used only by its name.
    published = PollQuerySet.as_manager()  # expect: 5 [unused-attribute]
    # An attribute that is not a field.
    unused_attribute = 1  # expect: 5 [unused-attribute]

    # Django reads the options of its inner `Meta` class.
    class Meta:
        ordering = ["title"]
        verbose_name = "poll"

    # Django calls `get_absolute_url` in the admin and in `redirect`.
    def get_absolute_url(self):
        return "/"

    def unused_method(self):  # expect: 9 [unused-method]
        return None


class Question(models.Model):
    objects = PollQuerySet.as_manager()
    text = models.TextField()


# Not a model: its `Meta` class and the attributes set to fields are not read.
class Plain:  # expect: 7 [unused-class]
    title = models.CharField(max_length=10)  # expect: 5 [unused-attribute] transitive

    class Meta:  # expect: 11 [unused-class] transitive
        ordering = ["title"]  # expect: 9 [unused-attribute] transitive

    def get_absolute_url(self):  # expect: 9 [unused-method] transitive
        return "/"
