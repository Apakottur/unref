class Class:
    on_class = 1
    unread_on_class = 2  # expect: unused attribute

    def __init__(self):  # expect: unused method
        self.on_instance = 3
        self.unread_on_instance = 4  # expect: unused attribute


instance = Class()
print(Class.on_class, instance.on_instance)
