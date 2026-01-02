// Example C++ file for testing GnawTreeWriter
#include <iostream>
#include <string>
#include <vector>
#include <memory>
#include <algorithm>

// Namespace definition
namespace banking {

// Class definition with inheritance
class Account {
protected:
    int id_;
    std::string name_;
    double balance_;
    
public:
    // Constructor
    Account(int id, const std::string& name) 
        : id_(id), name_(name), balance_(0.0) {}
    
    // Virtual destructor
    virtual ~Account() = default;
    
    // Getter methods
    int getId() const { return id_; }
    std::string getName() const { return name_; }
    double getBalance() const { return balance_; }
    
    // Virtual method
    virtual void deposit(double amount) {
        if (amount > 0) {
            balance_ += amount;
        }
    }
    
    virtual void withdraw(double amount) {
        if (amount > 0 && balance_ >= amount) {
            balance_ -= amount;
        }
    }
    
    // Pure virtual method
    virtual std::string getAccountType() const = 0;
    
    // Method with default parameter
    void print(bool detailed = false) const {
        std::cout << "Account ID: " << id_ << std::endl;
        std::cout << "Name: " << name_ << std::endl;
        std::cout << "Balance: $" << balance_ << std::endl;
        if (detailed) {
            std::cout << "Type: " << getAccountType() << std::endl;
        }
    }
};

// Derived class
class SavingsAccount : public Account {
private:
    double interestRate_;
    
public:
    SavingsAccount(int id, const std::string& name, double rate)
        : Account(id, name), interestRate_(rate) {}
    
    void applyInterest() {
        balance_ += balance_ * interestRate_;
    }
    
    std::string getAccountType() const override {
        return "Savings";
    }
};

// Another derived class
class CheckingAccount : public Account {
private:
    double overdraftLimit_;
    
public:
    CheckingAccount(int id, const std::string& name, double limit)
        : Account(id, name), overdraftLimit_(limit) {}
    
    void withdraw(double amount) override {
        if (amount > 0 && (balance_ + overdraftLimit_) >= amount) {
            balance_ -= amount;
        }
    }
    
    std::string getAccountType() const override {
        return "Checking";
    }
};

// Template class
template<typename T>
class Container {
private:
    std::vector<T> items_;
    
public:
    void add(const T& item) {
        items_.push_back(item);
    }
    
    size_t size() const {
        return items_.size();
    }
    
    T& operator[](size_t index) {
        return items_[index];
    }
    
    const T& operator[](size_t index) const {
        return items_[index];
    }
};

} // namespace banking

// Template function
template<typename T>
T max(T a, T b) {
    return (a > b) ? a : b;
}

// Main function with modern C++ features
int main() {
    using namespace banking;
    
    std::cout << "=== C++ Account Manager ===" << std::endl;
    
    // Smart pointers
    auto savings = std::make_unique<SavingsAccount>(1, "Alice Smith", 0.05);
    auto checking = std::make_unique<CheckingAccount>(2, "Bob Jones", 500.0);
    
    // Deposits
    savings->deposit(1000.0);
    checking->deposit(500.0);
    
    // Apply interest
    savings->applyInterest();
    
    // Print details
    savings->print(true);
    std::cout << std::endl;
    checking->print(true);
    std::cout << std::endl;
    
    // Using template function
    int maxInt = max(10, 20);
    double maxDouble = max(3.14, 2.71);
    std::cout << "Max int: " << maxInt << std::endl;
    std::cout << "Max double: " << maxDouble << std::endl;
    
    // Lambda expression
    auto printMessage = [](const std::string& msg) {
        std::cout << "Message: " << msg << std::endl;
    };
    printMessage("C++ is awesome!");
    
    // Range-based for loop with vector
    std::vector<int> numbers = {1, 2, 3, 4, 5};
    int sum = 0;
    for (const auto& num : numbers) {
        sum += num;
    }
    std::cout << "Sum of numbers: " << sum << std::endl;
    
    // Using STL algorithm
    auto it = std::find_if(numbers.begin(), numbers.end(), 
                           [](int n) { return n > 3; });
    if (it != numbers.end()) {
        std::cout << "First number > 3: " << *it << std::endl;
    }
    
    return 0;
}