from django import forms
from django.contrib import admin

from .models import Poll


# The metaclass of a form reads its fields, and a form calls its `clean_<field>` methods.
class ContactForm(forms.Form):
    email = forms.EmailField()
    unused_attribute = 1  # expect: 5 [unused-attribute]

    def clean_email(self):
        return self.cleaned_data["email"]

    # A form reads its `Media` class, but not a `Meta` class.
    class Media:
        js = ["contact.js"]

    class Meta:  # expect: 11 [unused-class]
        fields = ["email"]  # expect: 9 [unused-attribute] transitive


# A model form reads its `Meta` class.
class PollForm(forms.ModelForm):
    class Meta:
        model = Poll
        fields = ["title"]


class CalendarWidget(forms.Widget):
    class Media:
        css = {"all": ["calendar.css"]}


# The admin reads the `Media` class of its classes.
class PollAdmin(admin.ModelAdmin):
    class Media:
        js = ["admin.js"]


# Not a form.
class Validator:
    def clean_email(self):  # expect: 9 [unused-method]
        return None

    class Media:  # expect: 11 [unused-class]
        js = ["validator.js"]  # expect: 9 [unused-attribute] transitive


admin.site.register(Poll, PollAdmin)
FORMS = [ContactForm, PollForm, CalendarWidget, Validator]
