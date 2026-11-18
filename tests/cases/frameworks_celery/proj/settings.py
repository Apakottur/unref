from celery.schedules import crontab

# Celery beat runs a task by its name.
CELERY_BEAT_SCHEDULE = {
    "cleanup": {
        "task": "myapp.tasks.cleanup_old_sessions",
        "schedule": crontab(minute="0", hour="16"),
    },
}
