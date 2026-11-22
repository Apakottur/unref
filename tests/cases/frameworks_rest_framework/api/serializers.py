from django.db import models
from rest_framework import serializers
from rest_framework.serializers import ModelSerializer, SerializerMethodField


class Poll(models.Model):
    title = models.CharField(max_length=10)


class ChoiceSerializer(serializers.Serializer):
    text = serializers.CharField()


# The metaclass of a serializer reads its fields, which can be serializers, and a `ModelSerializer`
# reads its `Meta` class.
class PollSerializer(ModelSerializer):
    choices = ChoiceSerializer(many=True)
    votes = SerializerMethodField()
    unused_attribute = 1  # expect: 5 [unused-attribute]

    class Meta:
        model = Poll
        fields = ["title", "choices", "votes"]

    # A serializer calls the `validate_<field>` method of each field.
    def validate_title(self, value):
        return value

    # A `SerializerMethodField` calls the `get_<field>` method of its field.
    def get_votes(self, poll):
        return 0

    # Not the method of a `SerializerMethodField`.
    def get_title(self, poll):  # expect: 9 [unused-method]
        return ""

    def unused_method(self):  # expect: 9 [unused-method]
        return None


# Not a serializer.
class Plain:  # expect: 7 [unused-class]
    votes = SerializerMethodField()  # expect: 5 [unused-attribute] transitive

    class Meta:  # expect: 11 [unused-class] transitive
        fields = ["votes"]  # expect: 9 [unused-attribute] transitive

    def validate_title(self, value):  # expect: 9 [unused-method] transitive
        return value

    def get_votes(self, poll):  # expect: 9 [unused-method] transitive
        return 0
