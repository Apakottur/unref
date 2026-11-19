# Python runs a script with its own directory first on the import path, so an absolute import that
# only a module beside it can stand for imports that module.
import helpers_module
from helpers import warn

# A module of the project or of the environment comes first.
import json
from import_from_module_helpers import Class

if __name__ == "__main__":
    warn("no data"), helpers_module.used(), json.dumps({}), Class
