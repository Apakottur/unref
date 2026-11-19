import os

from celery import Celery

os.environ.setdefault("DJANGO_SETTINGS_MODULE", "proj.settings")

app = Celery("proj")
app.config_from_object("django.conf:settings", namespace="CELERY")
app.autodiscover_tasks()


# Celery's workers run the tasks that the decorators of an application register.
@app.task(bind=True, ignore_result=True)
def debug_task(self):
    self.request
