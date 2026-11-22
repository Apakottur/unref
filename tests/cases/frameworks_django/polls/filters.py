import django_filters

from .models import Poll


# The `FilterSet` of `django-filter` reads its filters and its `Meta` class.
class PollFilter(django_filters.FilterSet):
    title = django_filters.CharFilter(lookup_expr="icontains")
    unused_attribute = 1  # expect: 5 [unused-attribute]

    class Meta:
        model = Poll
        fields = ["title"]
