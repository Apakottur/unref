class Job:
    def run(self):  # expect: 9 [unused-method]
        return 1


def run():
    return 2


JOB = Job()  # expect: 1 [unused-variable]
