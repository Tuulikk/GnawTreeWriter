interface User {
    id: number;
    name: string;
    email: string;
    role: 'admin' | 'user' | 'guest';
}

interface AppState {
    users: User[];
    currentUser: User | null;
    isLoading: boolean;
}

function processUserData(users: User[]): User[] {
    return users.filter(user => user.email.includes('@'));
}

async function fetchUsers(): Promise<User[]> {
    const response = await fetch('/api/users');
    return response.json();
}

class UserService {
    private state: AppState = {
        users: [],
        currentUser: null,
        isLoading: false
    };

    async loadUsers(): Promise<void> {
        this.state.isLoading = true;
        try {
            this.state.users = await fetchUsers();
        } catch (error) {
            console.error('Failed to load users:', error);
        } finally {
            this.state.isLoading = false;
        }
    }

    getCurrentUser(): User | null {
        return this.state.currentUser;
    }
}

const service = new UserService();
service.loadUsers();
