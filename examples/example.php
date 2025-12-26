<?php

function greet($name) {
    return "Hello, " . $name . "!";
}

class Person {
    private $name;
    
    public function __construct($name) {
        $this->name = $name;
    }
    
    public function sayHello() {
        echo greet($this->name);
    }
}

$person = new Person("World");
$person->sayHello();
