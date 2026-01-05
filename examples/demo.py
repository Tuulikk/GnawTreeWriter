import os
import sys

class DatabaseManager:
    """A simple database manager for demo purposes."""

    def __init__(self, connection_string):
        self.connection_string = connection_string
        self.is_connected = False

    def connect(self):
        """Simulates connecting to a database."""
        print(f"Connecting to database at: {self.connection_string}")
        self.is_connected = True
        return True

    def disconnect(self):
        """Simulates disconnecting from a database."""
        if self.is_connected:
            print("Disconnecting from database...")
            self.is_connected = False
        return True

def authenticate_user(username, password):
    """
    Authentication logic for the demo.
    This is a target for semantic search testing.
    """
    # Security check: Log attempt
    print(f"Attempting authentication for user: {username}")
    if username == "admin" and password == "secret":
        print(f"User {username} authenticated successfully.")
        return True
    else:
        print("Authentication failed.")
        return False

def main():
    db_url = os.getenv("DATABASE_URL", "sqlite:///:memory:")
    db = DatabaseManager(db_url)

    if authenticate_user("admin", "secret"):
        db.connect()
        # Perform some operations
        db.disconnect()

if __name__ == "__main__":
    main()
