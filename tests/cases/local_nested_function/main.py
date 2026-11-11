# A nested function or class is a definition, so a read of it is a reference.
def function():
    def used():
        return 1

    def unused():  # expect: unused function
        return 2

    class Used:
        pass

    return used, Used


function()
