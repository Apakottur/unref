import celery
from celery import Task, current_app, shared_task

from proj.celery import app


@app.task
def cleanup_old_sessions() -> None:
    pass


@shared_task
def send_email() -> None:
    pass


@celery.shared_task(bind=True)
def retry_email(self) -> None:
    pass


@current_app.task
def refresh_cache() -> None:
    pass


# Celery calls the `run` method and the handlers of a task class.
class BaseTask(Task):
    def on_failure(self, exc, task_id, args, kwargs, exception_info):
        pass

    def run(self):
        pass

    def helper(self):  # expect: 9 [unused-method]
        pass


@app.task(base=BaseTask)
def with_base() -> None:
    pass


# An application made in a function, and one that a parameter is annotated as.
def make_worker():  # expect: 5 [unused-function]
    local = celery.Celery("local")

    @local.task
    def local_task() -> None:
        pass

    return local


def register(worker: celery.Celery) -> None:  # expect: 5 [unused-function]
    @worker.task
    def registered() -> None:
        pass


# Not a Celery object: a decorator with the same name is not a task.
class Registry:
    def task(self, function):
        return function


registry = Registry()


@registry.task
def not_a_task() -> None:  # expect: 5 [unused-function]
    pass
