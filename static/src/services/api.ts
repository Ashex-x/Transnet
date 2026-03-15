/**
 * API Service
 * Handles all HTTP communication with the backend
 * Base URL: /api (as per API documentation)
 */

import { AuthService } from './auth';

// API Response Types
export interface ApiResponse<T> {
  success: boolean;
  data?: T;
  error?: {
    code: string;
    message: string;
  };
}

// Translation Types
export interface TranslationRequest {
  text: string;
  source_lang: string;
  target_lang: string;
  mode?: 'basic' | 'explain';
  input_type?: 'auto' | 'word' | 'phrase' | 'sentence' | 'text';
}

export interface Translation {
  id: string;
  text: string;
  translation: string;
  source_lang: string;
  target_lang: string;
  input_type: string;
  provider: string;
  model: string;
  created_at: string;
  user_id?: string;
}

// Auth Types
export interface RegisterRequest {
  username: string;
  email: string;
  password: string;
}

export interface LoginRequest {
  email: string;
  password: string;
}

export interface User {
  uuid: string;
  username: string;
  email: string;
}

export interface AuthResponse {
  access_token: string;
  refresh_token: string;
  token_type: string;
  expires_in: number;
  user: User;
}

// History Types
export interface HistoryItem extends Translation {}

export interface HistoryResponse {
  translations: HistoryItem[];
  pagination: {
    page: number;
    limit: number;
    total: number;
    total_pages: number;
  };
}

export interface HistoryFilters {
  page?: number;
  limit?: number;
  source_lang?: string;
  target_lang?: string;
  input_type?: string;
  sort_by?: 'created_at' | 'source_lang';
  sort_order?: 'asc' | 'desc';
}

// Favorites Types
export interface Favorite {
  id: string;
  user_id: string;
  translation_id: string;
  note?: string;
  created_at: string;
  updated_at?: string;
  translation?: Translation;
  word_meaning?: WordMeaning;
}

export interface WordMeaning {
  id: string;
  word: string;
  pos: string;
  meaning: string;
  related_words: string[];
  source_lang: string;
  target_lang: string;
  word_type: string;
}

export interface CreateFavoriteRequest {
  translation_id: string;
  note?: string;
}

export interface FavoritesResponse {
  favorites: Favorite[];
  pagination: {
    page: number;
    limit: number;
    total: number;
    total_pages: number;
  };
}

export interface FavoriteFilters {
  page?: number;
  limit?: number;
  sort_by?: string;
  sort_order?: 'asc' | 'desc';
}

// Profile Types
export interface Profile {
  uuid: string;
  username: string;
  email: string;
  created_at: string;
  updated_at: string;
  stats: {
    total_translations: number;
    total_favorites: number;
    languages_used: string[];
  };
}

export interface UpdateProfileRequest {
  username?: string;
  email?: string;
}

// System Types
export interface SystemInfo {
  name: string;
  version: string;
  description: string;
  features: string[];
  supported_languages: string[];
  max_text_length: number;
}

export interface SystemStats {
  translations_today: number;
  active_users: number;
  translations_this_hour: number;
  llm_api_status: string;
  database_status: string;
  requests_per_minute: number;
  database_size_mb: number;
}

class ApiServiceClass {
  private baseUrl: string = '/api';

  private async request<T>(
    endpoint: string,
    options: RequestInit = {}
  ): Promise<ApiResponse<T>> {
    const url = `${this.baseUrl}${endpoint}`;
    
    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
      ...((options.headers as Record<string, string>) || {}),
    };

    // Add auth token if available
    const token = AuthService.getAccessToken();
    if (token) {
      headers['Authorization'] = `Bearer ${token}`;
    }

    try {
      const response = await fetch(url, {
        ...options,
        headers,
      });

      const data = await response.json();

      if (!response.ok) {
        // Handle 401 - try to refresh token
        if (response.status === 401 && token) {
          const refreshed = await AuthService.refreshToken();
          if (refreshed) {
            // Retry the request with new token
            return this.request(endpoint, options);
          }
        }
        
        return {
          success: false,
          error: data.error || { code: 'UNKNOWN_ERROR', message: 'An error occurred' },
        };
      }

      return data;
    } catch (error) {
      return {
        success: false,
        error: {
          code: 'NETWORK_ERROR',
          message: error instanceof Error ? error.message : 'Network error occurred',
        },
      };
    }
  }

  // ==================== SYSTEM ====================
  
  async getSystemInfo(): Promise<ApiResponse<SystemInfo>> {
    return this.request<SystemInfo>('/about');
  }

  async getSystemStats(): Promise<ApiResponse<SystemStats>> {
    return this.request<SystemStats>('/stats');
  }

  async healthCheck(): Promise<ApiResponse<{ status: string; service: string }>> {
    return this.request('/health');
  }

  // ==================== AUTHENTICATION ====================

  async register(request: RegisterRequest): Promise<ApiResponse<User>> {
    return this.request<User>('/account/register', {
      method: 'POST',
      body: JSON.stringify(request),
    });
  }

  async login(request: LoginRequest): Promise<ApiResponse<AuthResponse>> {
    return this.request<AuthResponse>('/account/login', {
      method: 'POST',
      body: JSON.stringify(request),
    });
  }

  async logout(): Promise<ApiResponse<{ message: string }>> {
    return this.request<{ message: string }>('/account/logout', {
      method: 'POST',
    });
  }

  async refreshToken(refreshToken: string): Promise<ApiResponse<{ access_token: string; token_type: string; expires_in: number }>> {
    return this.request('/account/refresh', {
      method: 'POST',
      body: JSON.stringify({ refresh_token: refreshToken }),
    });
  }

  async changePassword(currentPassword: string, newPassword: string): Promise<ApiResponse<{ message: string }>> {
    return this.request('/account/change-password', {
      method: 'POST',
      body: JSON.stringify({
        current_password: currentPassword,
        new_password: newPassword,
      }),
    });
  }

  // ==================== TRANSLATION ====================

  async translate(request: TranslationRequest): Promise<ApiResponse<Translation>> {
    return this.request<Translation>('/translate', {
      method: 'POST',
      body: JSON.stringify(request),
    });
  }

  // ==================== HISTORY ====================

  async getHistory(filters: HistoryFilters = {}): Promise<ApiResponse<HistoryResponse>> {
    const params = new URLSearchParams();
    
    if (filters.page) params.set('page', filters.page.toString());
    if (filters.limit) params.set('limit', filters.limit.toString());
    if (filters.source_lang) params.set('source_lang', filters.source_lang);
    if (filters.target_lang) params.set('target_lang', filters.target_lang);
    if (filters.input_type) params.set('input_type', filters.input_type);
    if (filters.sort_by) params.set('sort_by', filters.sort_by);
    if (filters.sort_order) params.set('sort_order', filters.sort_order);

    const query = params.toString();
    return this.request<HistoryResponse>(`/history${query ? `?${query}` : ''}`);
  }

  async getHistoryItem(id: string): Promise<ApiResponse<HistoryItem>> {
    return this.request<HistoryItem>(`/history/${id}`);
  }

  async deleteHistoryItem(id: string): Promise<ApiResponse<{ message: string }>> {
    return this.request<{ message: string }>(`/history/${id}`, {
      method: 'DELETE',
    });
  }

  // ==================== FAVORITES ====================

  async getFavorites(filters: FavoriteFilters = {}): Promise<ApiResponse<FavoritesResponse>> {
    const params = new URLSearchParams();
    
    if (filters.page) params.set('page', filters.page.toString());
    if (filters.limit) params.set('limit', filters.limit.toString());
    if (filters.sort_by) params.set('sort_by', filters.sort_by);
    if (filters.sort_order) params.set('sort_order', filters.sort_order);

    const query = params.toString();
    return this.request<FavoritesResponse>(`/favorites${query ? `?${query}` : ''}`);
  }

  async addFavorite(request: CreateFavoriteRequest): Promise<ApiResponse<Favorite>> {
    return this.request<Favorite>('/favorites', {
      method: 'POST',
      body: JSON.stringify(request),
    });
  }

  async updateFavorite(id: string, note: string): Promise<ApiResponse<Favorite>> {
    return this.request<Favorite>(`/favorites/${id}`, {
      method: 'PUT',
      body: JSON.stringify({ note }),
    });
  }

  async deleteFavorite(id: string): Promise<ApiResponse<{ message: string }>> {
    return this.request<{ message: string }>(`/favorites/${id}`, {
      method: 'DELETE',
    });
  }

  // ==================== PROFILE ====================

  async getProfile(): Promise<ApiResponse<Profile>> {
    return this.request<Profile>('/profile');
  }

  async updateProfile(request: UpdateProfileRequest): Promise<ApiResponse<Profile>> {
    return this.request<Profile>('/profile', {
      method: 'PUT',
      body: JSON.stringify(request),
    });
  }
}

export const ApiService = new ApiServiceClass();
