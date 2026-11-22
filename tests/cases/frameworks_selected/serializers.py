from rest_framework import serializers


# The rules of Django REST framework are turned off.
class PollSerializer(serializers.Serializer):  # expect: 7 [unused-class]
    title = serializers.CharField()  # expect: 5 [unused-attribute] transitive

    class Meta:  # expect: 11 [unused-class] transitive
        fields = ["title"]  # expect: 9 [unused-attribute] transitive
