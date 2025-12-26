#!/usr/bin/env python3

def greet(name):
    return f"Hello, {name}!"

class Person:
    def __init__(self, name):
        self.name = name
    
    def say_hello(self):
        print(greet(self.name))

if __name__ == "__main__":
    person = Person("World")
    person.say_hello()