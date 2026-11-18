# Split settings: the settings module star imports the module of the shared settings.
from pathlib import Path

BASE_DIR = Path(__file__).resolve().parent.parent
ROOT_URLCONF = "mysite.urls"
