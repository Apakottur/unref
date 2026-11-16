# A read of a name of the module is not a reference to a method or attribute of that name.
class Job:
    def run(self):  # expect: 9 [unused-method]
        return 1

    def stop(self):  # expect: 9 [unused-method]
        return 2


def run():
    return 3


run()
JOB = Job()  # expect: 1 [unused-variable]

VALUE = 1


class Config:
    VALUE = 2  # expect: 5 [unused-attribute]


print(VALUE, Config)
