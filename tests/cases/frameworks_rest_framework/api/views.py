from rest_framework import generics, permissions, viewsets
from rest_framework.decorators import action
from rest_framework.response import Response
from rest_framework.views import APIView

from .serializers import Poll, PollSerializer


# A view calls the method named by the HTTP method of a request, and reads the attributes that
# configure it or a filter backend.
class PollView(APIView):
    permission_classes = [permissions.IsAuthenticated]
    throttle_scope = "polls"
    unused_attribute = 1  # expect: 5 [unused-attribute]

    def get(self, request):
        return Response()

    def unused_method(self):  # expect: 9 [unused-method]
        return None


class PollList(generics.ListCreateAPIView):
    queryset = Poll.objects.all()
    serializer_class = PollSerializer
    search_fields = ["title"]
    filterset_fields = ["title"]


# The routers route requests to the actions of a viewset, and to the methods that `action` registers.
class PollViewSet(viewsets.ViewSet):
    def list(self, request):
        return Response()

    def retrieve(self, request, pk=None):
        return Response()

    @action(detail=True, methods=["post"])
    def vote(self, request, pk=None):
        return Response()

    def unused_method(self):  # expect: 9 [unused-method]
        return None


# Not a viewset: a view has no actions.
class PollDetail(generics.GenericAPIView):
    def retrieve(self, request, pk=None):  # expect: 9 [unused-method]
        return Response()


# Not a view.
class Plain:  # expect: 7 [unused-class]
    serializer_class = PollSerializer  # expect: 5 [unused-attribute] transitive

    def get(self, request):  # expect: 9 [unused-method] transitive
        return None

    def list(self, request):  # expect: 9 [unused-method] transitive
        return None


VIEWS = [PollView, PollList, PollViewSet, PollDetail]  # expect: 1 [unused-variable]
