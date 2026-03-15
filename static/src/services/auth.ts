/**
 * AuthService
 * Manages JWT tokens and authentication state
 */

import { ApiService, AuthResponse, User } from './api';

const STORAGE_KEY = 'transnet_auth';

interface StoredAuth {
  access_token: string;
  refresh_token: string;
  expires_at: number;
  user: User;
}

class AuthServiceClass {
  private currentUser: User | null = null;
  private listeners: Set<(isAuthenticated: boolean) => void> = new Set();

  constructor() {
    this.loadFromStorage();
  }

  // ==================== STORAGE ====================

  private loadFromStorage(): void {
    try {
      const stored = localStorage.getItem(STORAGE_KEY);
      if (stored) {
        const auth: StoredAuth = JSON.parse(stored);
        // Check if token is expired
        if (auth.expires_at > Date.now()) {
          this.currentUser = auth.user;
        } else {
          // Token expired, clear storage
          this.clearStorage();
        }
      }
    } catch {
      this.clearStorage();
    }
  }

  private saveToStorage(auth: StoredAuth): void {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(auth));
  }

  private clearStorage(): void {
    localStorage.removeItem(STORAGE_KEY);
    this.currentUser = null;
  }

  // ==================== TOKEN MANAGEMENT ====================

  getAccessToken(): string | null {
    try {
      const stored = localStorage.getItem(STORAGE_KEY);
      if (stored) {
        const auth: StoredAuth = JSON.parse(stored);
        return auth.access_token;
      }
    } catch {
      return null;
    }
    return null;
  }

  getRefreshToken(): string | null {
    try {
      const stored = localStorage.getItem(STORAGE_KEY);
      if (stored) {
        const auth: StoredAuth = JSON.parse(stored);
        return auth.refresh_token;
      }
    } catch {
      return null;
    }
    return null;
  }

  async refreshToken(): Promise<boolean> {
    const refreshToken = this.getRefreshToken();
    if (!refreshToken) {
      this.logout();
      return false;
    }

    const response = await ApiService.refreshToken(refreshToken);
    
    if (response.success && response.data) {
      const stored = localStorage.getItem(STORAGE_KEY);
      if (stored) {
        const auth: StoredAuth = JSON.parse(stored);
        auth.access_token = response.data.access_token;
        auth.expires_at = Date.now() + response.data.expires_in * 1000;
        this.saveToStorage(auth);
      }
      return true;
    } else {
      this.logout();
      return false;
    }
  }

  // ==================== AUTHENTICATION ====================

  async login(email: string, password: string): Promise<{ success: boolean; error?: string }> {
    const response = await ApiService.login({ email, password });

    if (response.success && response.data) {
      this.setAuth(response.data);
      this.notifyListeners(true);
      return { success: true };
    } else {
      return { 
        success: false, 
        error: response.error?.message || 'Login failed' 
      };
    }
  }

  async register(username: string, email: string, password: string): Promise<{ success: boolean; error?: string }> {
    const response = await ApiService.register({ username, email, password });

    if (response.success) {
      return { success: true };
    } else {
      return { 
        success: false, 
        error: response.error?.message || 'Registration failed' 
      };
    }
  }

  async logout(): Promise<void> {
    // Call logout endpoint (token will be invalidated on server)
    await ApiService.logout();
    
    // Clear local storage
    this.clearStorage();
    
    // Notify listeners
    this.notifyListeners(false);
  }

  private setAuth(authResponse: AuthResponse): void {
    const auth: StoredAuth = {
      access_token: authResponse.access_token,
      refresh_token: authResponse.refresh_token,
      expires_at: Date.now() + authResponse.expires_in * 1000,
      user: authResponse.user,
    };
    
    this.currentUser = authResponse.user;
    this.saveToStorage(auth);
  }

  // ==================== STATE ====================

  isAuthenticated(): boolean {
    return this.currentUser !== null;
  }

  getCurrentUser(): User | null {
    return this.currentUser;
  }

  // ==================== LISTENERS ====================

  onAuthChange(callback: (isAuthenticated: boolean) => void): () => void {
    this.listeners.add(callback);
    
    // Return unsubscribe function
    return () => {
      this.listeners.delete(callback);
    };
  }

  private notifyListeners(isAuthenticated: boolean): void {
    this.listeners.forEach(callback => callback(isAuthenticated));
  }

  // ==================== UTILITIES ====================

  async updateUserInfo(): Promise<void> {
    if (!this.isAuthenticated()) return;

    const response = await ApiService.getProfile();
    if (response.success && response.data) {
      this.currentUser = {
        uuid: response.data.uuid,
        username: response.data.username,
        email: response.data.email,
      };
      
      // Update storage
      const stored = localStorage.getItem(STORAGE_KEY);
      if (stored) {
        const auth: StoredAuth = JSON.parse(stored);
        auth.user = this.currentUser;
        this.saveToStorage(auth);
      }
      
      this.notifyListeners(true);
    }
  }
}

export const AuthService = new AuthServiceClass();
