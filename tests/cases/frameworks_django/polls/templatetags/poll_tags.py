from django import template

# Django loads a template library by its `register` variable, and templates use the tags and filters
# that it registers by their names.
register = template.Library()


@register.filter
def lower(value):
    return value.lower()


@register.filter(name="upper")
def upper(value):
    return value.upper()


@register.simple_tag
def current_time():
    return ""


@register.inclusion_tag("results.html")
def show_results(poll):
    return {}


def unused_function():  # expect: 5 [unused-function]
    return None


# Not a template library: a decorator with the same name does not register a tag.
class Registry:
    def filter(self, function):
        return function


other = Registry()


@other.filter
def not_a_filter(value):  # expect: 5 [unused-function]
    return value
