from django.views import View, generic

from .filters import PollFilter
from .forms import FORMS
from .models import Poll, Question


def index(request):
    return None


def not_found(request, exception):
    return None


# A class-based view calls the method named by the HTTP method of a request.
class PollView(View):
    def get(self, request):
        return None

    def post(self, request):
        return None

    def unused_method(self):  # expect: 9 [unused-method]
        return None


class PollListView(generic.ListView):
    def get(self, request):
        return None


VIEWS = [PollView, PollListView, PollFilter, FORMS, Poll, Question]  # expect: 1 [unused-variable]
