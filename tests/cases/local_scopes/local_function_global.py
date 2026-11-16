# A function that is declared global is a function of the module.
def install():
    global helper

    def helper():
        return 1


install()
print(helper())
