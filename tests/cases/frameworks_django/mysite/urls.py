from django.urls import include, path

from polls import views

# Django reads `urlpatterns` and the error handlers from the module that `ROOT_URLCONF` names.
urlpatterns = [
    path("polls/", include("polls.urls")),
]
handler404 = views.not_found

# Not a name that Django reads.
app_name = "mysite"  # expect: 1 [unused-variable]
