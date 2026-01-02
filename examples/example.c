// Example C file for testing GnawTreeWriter
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define MAX_SIZE 100
#define PI 3.14159

// Structure definition
typedef struct {
    int id;
    char name[50];
    float balance;
} Account;

// Function declarations
int add(int a, int b);
void print_account(Account *acc);
Account* create_account(int id, const char *name);

// Global variable
static int account_counter = 0;

// Function to add two numbers
int add(int a, int b) {
    return a + b;
}

// Function to create a new account
Account* create_account(int id, const char *name) {
    Account *acc = (Account*)malloc(sizeof(Account));
    if (acc == NULL) {
        return NULL;
    }
    
    acc->id = id;
    strncpy(acc->name, name, sizeof(acc->name) - 1);
    acc->name[sizeof(acc->name) - 1] = '\0';
    acc->balance = 0.0;
    
    account_counter++;
    return acc;
}

// Function to print account details
void print_account(Account *acc) {
    if (acc == NULL) {
        printf("Invalid account\n");
        return;
    }
    
    printf("Account ID: %d\n", acc->id);
    printf("Name: %s\n", acc->name);
    printf("Balance: %.2f\n", acc->balance);
}

// Main function
int main(int argc, char *argv[]) {
    printf("Welcome to Account Manager\n");
    
    // Create sample account
    Account *acc = create_account(1, "John Doe");
    if (acc == NULL) {
        fprintf(stderr, "Failed to create account\n");
        return 1;
    }
    
    // Update balance
    acc->balance = 1000.50;
    
    // Print account details
    print_account(acc);
    
    // Test add function
    int sum = add(10, 20);
    printf("Sum: %d\n", sum);
    
    // Cleanup
    free(acc);
    
    printf("Total accounts created: %d\n", account_counter);
    return 0;
}