from django.urls import path

from . import views

# Django reads `urlpatterns` and `app_name` from a module that `include` names.
app_name = "polls"
urlpatterns = [
    path("", views.index),
]

# It reads the error handlers from the module of the root URLs only.
handler500 = views.not_found  # expect: 1 [unused-variable]
