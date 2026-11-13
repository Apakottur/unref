class Class:
    on_class = 1
    unread_on_class = 2  # expect: 5 [unused-attribute]

    def __init__(self):
        self.on_instance = 3
        self.unread_on_instance = 4  # expect: 14 [unused-attribute]


instance = Class()
print(Class.on_class, instance.on_instance)
