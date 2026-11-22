from django.db.models.signals import post_save
from django.dispatch import receiver

from .models import Poll


# A signal calls the receivers that `receiver` connects.
@receiver(post_save, sender=Poll)
def poll_saved(sender, instance, **kwargs):
    pass


def unused_function(sender, **kwargs):  # expect: 5 [unused-function]
    pass
