# Not an application of FastAPI.
class Registry:
    def get(self, path):
        return lambda function: function


app = Registry()
