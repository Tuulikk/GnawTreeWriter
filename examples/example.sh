#!/bin/bash
# Example Bash script for testing GnawTreeWriter

set -e  # Exit on error
set -u  # Exit on undefined variable

# Global variables
SCRIPT_NAME="$(basename "$0")"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
LOG_FILE="/tmp/example.log"
COUNTER=0

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print colored messages
print_message() {
    local color="$1"
    local message="$2"
    echo -e "${color}${message}${NC}"
}

# Function to log messages
log() {
    local message="$1"
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $message" >> "$LOG_FILE"
    echo "$message"
}

# Function to check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Function to add two numbers
add() {
    local a="$1"
    local b="$2"
    echo $((a + b))
}

# Function to process file
process_file() {
    local file="$1"

    if [[ ! -f "$file" ]]; then
        print_message "$RED" "Error: File '$file' not found"
        return 1
    fi

    log "Processing file: $file"

    # Count lines
    local line_count=$(wc -l < "$file")
    print_message "$GREEN" "Lines: $line_count"

    # Count words
    local word_count=$(wc -w < "$file")
    print_message "$GREEN" "Words: $word_count"

    return 0
}

# Function with array parameter
print_array() {
    local -n arr=$1  # nameref
    local i

    for i in "${!arr[@]}"; do
        echo "  [$i]: ${arr[$i]}"
    done
}

# Main function
main() {
    print_message "$YELLOW" "=== Bash Script Example ==="

    # Check required commands
    if ! command_exists "git"; then
        print_message "$RED" "Warning: git not found"
    fi

    # Array example
    local fruits=("apple" "banana" "cherry" "date")
    print_message "$GREEN" "Fruits array:"
    print_array fruits

    # Associative array
    declare -A person
    person[name]="John Doe"
    person[age]="30"
    person[city]="New York"

    print_message "$GREEN" "Person details:"
    for key in "${!person[@]}"; do
        echo "  $key: ${person[$key]}"
    done

    # Arithmetic operations
    local result=$(add 10 20)
    log "Result of 10 + 20 = $result"

    # Conditional statement
    if [[ $result -gt 25 ]]; then
        print_message "$GREEN" "Result is greater than 25"
    else
        print_message "$YELLOW" "Result is 25 or less"
    fi

    # Loop examples
    print_message "$YELLOW" "Counting to 5:"
    for i in {1..5}; do
        echo -n "$i "
        ((COUNTER++))
    done
    echo

    # While loop
    local count=0
    while [[ $count -lt 3 ]]; do
        log "Loop iteration: $count"
        ((count++))
    done

    # Case statement
    local option="start"
    case "$option" in
        start)
            print_message "$GREEN" "Starting process..."
            ;;
        stop)
            print_message "$RED" "Stopping process..."
            ;;
        restart)
            print_message "$YELLOW" "Restarting process..."
            ;;
        *)
            print_message "$RED" "Unknown option: $option"
            ;;
    esac

    # String manipulation
    local text="Hello, World!"
    print_message "$GREEN" "Original: $text"
    print_message "$GREEN" "Uppercase: ${text^^}"
    print_message "$GREEN" "Lowercase: ${text,,}"
    print_message "$GREEN" "Length: ${#text}"

    # File operations
    if [[ -f "$0" ]]; then
        print_message "$GREEN" "Processing current script..."
        process_file "$0" || true
    fi

    # Command substitution
    local current_date=$(date '+%Y-%m-%d')
    local current_user=$(whoami)
    log "Script executed by $current_user on $current_date"

    # Exit status
    print_message "$GREEN" "Script completed successfully!"
    print_message "$GREEN" "Total counter: $COUNTER"

    return 0
}

# Trap for cleanup
cleanup() {
    log "Cleaning up..."
    # Add cleanup tasks here
}

trap cleanup EXIT

# Run main function with all arguments
main "$@"

exit $?
