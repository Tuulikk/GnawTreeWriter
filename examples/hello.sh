#!/bin/bash

# Simple Bash script for testing

greet() {
    local name="$1"
    echo "Hello, $name!"
}

add() {
    local a=$1
    local b=$2
    echo $((a + b))
}

main() {
    echo "Hello, World!"

    greet "Bash"

    result=$(add 5 3)
    echo "5 + 3 = $result"

    # Loop example
    for i in {1..3}; do
        echo "Count: $i"
    done
}

# Run main function
main
