/**
 * API Service
 * Handles all HTTP communication with the backend
 * Base URL: /api (as per API documentation)
 */

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
  mode?: 'basic' | 'explain' | 'full_analysis';
  input_type?: 'auto' | 'word' | 'phrase' | 'sentence' | 'paragraph' | 'essay' | 'text';
}

// Translation Data Types (from docs/types.md)

interface TranslationExample {
  source: string;
  translation: string;
}

interface Explain {
  meaning: string;
  story: string;
  when_to_use: string;
  how_to_use: string;
  context: string;
  lexical_analysis: {
    root?: string;
    structure?: string;
    idiomatic?: boolean;
    related_phrases?: string[];
  };
}

interface RelatedWord {
  word: string;
  type: string;
  similarity: number;
}

interface Relationships {
  related_words?: RelatedWord[];
  related_phrases?: Array<{
    phrase: string;
    type: string;
    similarity: number;
  }>;
  related_concepts?: string[];
  by_pos?: {
    nouns?: string[];
    verbs?: string[];
    adjectives?: string[];
  };
}

// Word Translation Types
export interface TranslationWordBasic {
  headword: string;
  part_of_speech: string;
  phonetic: string;
  translations: string[];
  synonyms: string[];
  antonyms: string[];
  examples: TranslationExample[];
}

export interface TranslationWordExplain extends TranslationWordBasic {
  explain: Explain;
}

export interface TranslationWordFullAnalysis extends TranslationWordExplain {
  relationships: Relationships;
}

// Phrase Translation Types
export interface TranslationPhraseBasic {
  phrase: string;
  headword: string;
  part_of_speech: string;
  translations: string[];
  examples: TranslationExample[];
}

export interface TranslationPhraseExplain extends TranslationPhraseBasic {
  explain: Explain;
}

export interface TranslationPhraseFullAnalysis extends TranslationPhraseExplain {
  relationships: Relationships;
}

// Sentence Translation Types
export interface TranslationSentenceBasic {
  tone: string;
  rephrasing: string;
}

export interface TranslationSentenceExplain extends TranslationSentenceBasic {
  explain: {
    meaning: string;
    usage: string;
    context: string;
  };
}

// Paragraph/Essay Translation Type (shared for both)
export interface TranslationParagraphEssayBasic {
  text: string;
  translation: string;
}

// Discriminated union for all translation data types
export type TranslationData =
  | TranslationWordBasic
  | TranslationWordExplain
  | TranslationWordFullAnalysis
  | TranslationPhraseBasic
  | TranslationPhraseExplain
  | TranslationPhraseFullAnalysis
  | TranslationSentenceBasic
  | TranslationSentenceExplain
  | TranslationParagraphEssayBasic;

// Type guards for translation data
export function isTranslationWordBasic(data: TranslationData): data is TranslationWordBasic {
  return 'headword' in data && !('explain' in data);
}

export function isTranslationWordExplain(data: TranslationData): data is TranslationWordExplain {
  return 'headword' in data && 'explain' in data && !('relationships' in data);
}

export function isTranslationWordFullAnalysis(data: TranslationData): data is TranslationWordFullAnalysis {
  return 'headword' in data && 'explain' in data && 'relationships' in data;
}

export function isTranslationPhraseBasic(data: TranslationData): data is TranslationPhraseBasic {
  return 'phrase' in data && !('explain' in data);
}

export function isTranslationPhraseExplain(data: TranslationData): data is TranslationPhraseExplain {
  return 'phrase' in data && 'explain' in data && !('relationships' in data);
}

export function isTranslationPhraseFullAnalysis(data: TranslationData): data is TranslationPhraseFullAnalysis {
  return 'phrase' in data && 'explain' in data && 'relationships' in data;
}

export function isTranslationSentenceBasic(data: TranslationData): data is TranslationSentenceBasic {
  return 'rephrasing' in data && !('explain' in data);
}

export function isTranslationSentenceExplain(data: TranslationData): data is TranslationSentenceExplain {
  return 'rephrasing' in data && 'explain' in data;
}

export function isTranslationParagraphEssayBasic(data: TranslationData): data is TranslationParagraphEssayBasic {
  return 'translation' in data && 'text' in data;
}

export interface Translation {
  translation_id: string;
  text: string;
  source_lang: string;
  target_lang: string;
  input_type: 'word' | 'phrase' | 'sentence' | 'paragraph' | 'essay';
  provider: string;
  model: string;
  translation: TranslationData;
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
  user_id: string;
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
  user_id: string;
  translation_id: string;
  note?: string;
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
  user_id: string;
  username: string;
  email: string;
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
  private accessToken: string | null = null;

  /**
   * Set access token for API requests
   */
  setAccessToken(token: string): void {
    this.accessToken = token;
  }

  /**
   * Clear access token
   */
  clearAccessToken(): void {
    this.accessToken = null;
  }

  private async request<T>(
    endpoint: string,
    options: RequestInit = {}
  ): Promise<ApiResponse<T>> {
    const url = `${this.baseUrl}${endpoint}`;
    const method = options.method || 'GET';
    
    // Log the request
    console.log(`[API] ${method} ${url}`);
    
    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
      ...((options.headers as Record<string, string>) || {}),
    };

    // Add auth token if available
    if (this.accessToken) {
      headers['Authorization'] = `Bearer ${this.accessToken}`;
    }

    try {
      const response = await fetch(url, {
        ...options,
        headers,
      });

      const data = await response.json();

      // Log the response status code
      console.log(`[API] ${method} ${url} - ${response.status} ${response.statusText}`);

      if (!response.ok) {
        return {
          success: false,
          error: data.error || { code: 'UNKNOWN_ERROR', message: 'An error occurred' },
        };
      }

      return data;
    } catch (error) {
      // Log network errors
      console.error(`[API] ${method} ${url} - Network Error:`, error);
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
    return this.request<Translation>('/transnet/translate', {
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
    return this.request<HistoryResponse>(`/transnet/history${query ? `?${query}` : ''}`);
  }

  async getHistoryItem(translation_id: string): Promise<ApiResponse<HistoryItem>> {
    return this.request<HistoryItem>(`/transnet/history/${translation_id}`);
  }

  async deleteHistoryItem(translation_id: string): Promise<ApiResponse<{ message: string }>> {
    return this.request<{ message: string }>(`/transnet/history/${translation_id}`, {
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
    return this.request<FavoritesResponse>(`/transnet/favorites${query ? `?${query}` : ''}`);
  }

  async addFavorite(request: CreateFavoriteRequest): Promise<ApiResponse<Favorite>> {
    return this.request<Favorite>('/transnet/favorites', {
      method: 'POST',
      body: JSON.stringify(request),
    });
  }

  async updateFavorite(translation_id: string, note: string): Promise<ApiResponse<Favorite>> {
    return this.request<Favorite>(`/transnet/favorites/${translation_id}`, {
      method: 'PUT',
      body: JSON.stringify({ note }),
    });
  }

  async deleteFavorite(translation_id: string): Promise<ApiResponse<{ message: string }>> {
    return this.request<{ message: string }>(`/transnet/favorites/${translation_id}`, {
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
