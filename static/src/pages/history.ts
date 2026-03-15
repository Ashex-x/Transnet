/**
 * History Page
 * Displays user's translation history
 */

import { ParticleBackground } from '../components/ParticleBackground';
import { Header } from '../components/Header';
import { ApiService, HistoryItem, HistoryResponse } from '../services/api';
import { AuthService } from '../services/auth';
import { Toast } from '../components/Toast';
import { router } from '../router';

const LANGUAGES = [
  { code: 'en', name: 'English' },
  { code: 'es', name: 'Español' },
  { code: 'fr', name: 'Français' },
  { code: 'de', name: 'Deutsch' },
  { code: 'zh', name: '中文' },
  { code: 'ja', name: '日本語' },
];

export class History {
  private container: HTMLElement;
  private particleBg: ParticleBackground | null = null;
  private header: Header | null = null;
  private historyData: HistoryResponse | null = null;
  private currentPage = 1;
  private filters = {
    source_lang: '',
    target_lang: '',
    input_type: '',
  };

  constructor(container: HTMLElement = document.body) {
    this.container = container;
    this.container.innerHTML = '';
  }

  async render(): Promise<void> {
    // Check authentication
    if (!AuthService.isAuthenticated()) {
      Toast.info('请先登录');
      router.navigate('/login');
      return;
    }

    // Add particle background
    this.particleBg = new ParticleBackground(this.container);

    // Add header
    this.header = new Header();
    this.header.mount(this.container);

    // Create main content
    const main = document.createElement('main');
    main.className = 'profile-page';
    main.innerHTML = `
      <div class="container">
        <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 32px;">
          <h1 style="font-size: 2rem; font-weight: 700;">Translation History</h1>
          <div style="display: flex; gap: 12px;">
            <select class="input-field filter-source" style="width: 140px; padding: 8px 12px;">
              <option value="">Source</option>
              ${LANGUAGES.map(lang => `<option value="${lang.code}">${lang.name}</option>`).join('')}
            </select>
            <select class="input-field filter-target" style="width: 140px; padding: 8px 12px;">
              <option value="">Target</option>
              ${LANGUAGES.map(lang => `<option value="${lang.code}">${lang.name}</option>`).join('')}
            </select>
            <select class="input-field filter-type" style="width: 120px; padding: 8px 12px;">
              <option value="">Type</option>
              <option value="word">Word</option>
              <option value="phrase">Phrase</option>
              <option value="sentence">Sentence</option>
              <option value="text">Text</option>
            </select>
          </div>
        </div>
        
        <div class="history-list"></div>
        
        <div class="pagination"></div>
      </div>
    `;

    this.container.appendChild(main);
    this.bindFilterEvents();
    await this.loadHistory();
  }

  private bindFilterEvents(): void {
    const sourceFilter = this.container.querySelector('.filter-source');
    const targetFilter = this.container.querySelector('.filter-target');
    const typeFilter = this.container.querySelector('.filter-type');

    sourceFilter?.addEventListener('change', (e) => {
      this.filters.source_lang = (e.target as HTMLSelectElement).value;
      this.currentPage = 1;
      this.loadHistory();
    });

    targetFilter?.addEventListener('change', (e) => {
      this.filters.target_lang = (e.target as HTMLSelectElement).value;
      this.currentPage = 1;
      this.loadHistory();
    });

    typeFilter?.addEventListener('change', (e) => {
      this.filters.input_type = (e.target as HTMLSelectElement).value;
      this.currentPage = 1;
      this.loadHistory();
    });
  }

  private async loadHistory(): Promise<void> {
    const response = await ApiService.getHistory({
      page: this.currentPage,
      limit: 10,
      ...this.filters,
      sort_by: 'created_at',
      sort_order: 'desc',
    });

    if (response.success && response.data) {
      this.historyData = response.data;
      this.renderHistoryList();
      this.renderPagination();
    } else {
      Toast.error('Failed to load history');
    }
  }

  private renderHistoryList(): void {
    const container = this.container.querySelector('.history-list');
    if (!container) return;

    const items = this.historyData?.translations || [];

    if (items.length === 0) {
      container.innerHTML = `
        <div class="empty-state">
          <div class="empty-state__icon">📜</div>
          <h3 class="empty-state__title">No History Yet</h3>
          <p class="empty-state__description">Start translating and your history will appear here</p>
        </div>
      `;
      return;
    }

    container.innerHTML = items.map((item, index) => this.renderHistoryCard(item, index)).join('');

    // Bind action events
    container.querySelectorAll('.history-copy-btn').forEach(btn => {
      btn.addEventListener('click', (e) => {
        e.stopPropagation();
        const text = btn.getAttribute('data-text');
        if (text) {
          navigator.clipboard.writeText(text);
          Toast.success('Copied');
        }
      });
    });

    container.querySelectorAll('.history-favorite-btn').forEach(btn => {
      btn.addEventListener('click', (e) => {
        e.stopPropagation();
        const id = btn.getAttribute('data-id');
        if (id) this.addToFavorite(id);
      });
    });

    container.querySelectorAll('.history-delete-btn').forEach(btn => {
      btn.addEventListener('click', (e) => {
        e.stopPropagation();
        const id = btn.getAttribute('data-id');
        if (id) this.deleteHistoryItem(id);
      });
    });
  }

  private renderHistoryCard(item: HistoryItem, index: number): string {
    return `
      <div class="glass-card history-card animate-fade-in-up" style="animation-delay: ${index * 0.05}s;">
        <div class="history-card__timeline">
          <div class="history-card__dot"></div>
          <div class="history-card__line"></div>
        </div>
        <div class="history-card__content">
          <div class="history-card__header">
            <div style="font-size: 0.85rem; color: var(--text-tertiary);">${this.formatDate(item.created_at)}</div>
            <div class="history-card__actions">
              <button class="btn btn--ghost btn--icon history-copy-btn" data-text="${this.escapeHtml(item.translation)}" title="复制">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
                  <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
                </svg>
              </button>
              <button class="btn btn--ghost btn--icon history-favorite-btn" data-id="${item.id}" title="收藏">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"></polygon>
                </svg>
              </button>
              <button class="btn btn--ghost btn--icon history-delete-btn" data-id="${item.id}" title="删除">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <polyline points="3 6 5 6 21 6"></polyline>
                  <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path>
                </svg>
              </button>
            </div>
          </div>
          <div class="history-card__text">${this.escapeHtml(item.text)}</div>
          <div class="history-card__translation">${this.escapeHtml(item.translation)}</div>
          <div class="history-card__meta">
            <span class="history-card__badge">${item.source_lang.toUpperCase()} → ${item.target_lang.toUpperCase()}</span>
            <span>${item.input_type}</span>
          </div>
        </div>
      </div>
    `;
  }

  private renderPagination(): void {
    const container = this.container.querySelector('.pagination');
    if (!container || !this.historyData) return;

    const { page, total_pages } = this.historyData.pagination;
    
    if (total_pages <= 1) {
      container.innerHTML = '';
      return;
    }

    let pages: (number | string)[] = [];
    
    if (total_pages <= 7) {
      pages = Array.from({ length: total_pages }, (_, i) => i + 1);
    } else {
      if (page <= 3) {
        pages = [1, 2, 3, 4, '...', total_pages];
      } else if (page >= total_pages - 2) {
        pages = [1, '...', total_pages - 3, total_pages - 2, total_pages - 1, total_pages];
      } else {
        pages = [1, '...', page - 1, page, page + 1, '...', total_pages];
      }
    }

    container.innerHTML = `
      <button class="pagination__btn" ${page === 1 ? 'disabled' : ''} data-page="${page - 1}">←</button>
      ${pages.map(p => {
        if (p === '...') return `<span class="pagination__btn" style="cursor: default;">...</span>`;
        return `<button class="pagination__btn ${p === page ? 'active' : ''}" data-page="${p}">${p}</button>`;
      }).join('')}
      <button class="pagination__btn" ${page === total_pages ? 'disabled' : ''} data-page="${page + 1}">→</button>
    `;

    container.querySelectorAll('button[data-page]').forEach(btn => {
      btn.addEventListener('click', () => {
        const pageNum = parseInt(btn.getAttribute('data-page') || '1');
        this.currentPage = pageNum;
        this.loadHistory();
        window.scrollTo({ top: 0, behavior: 'smooth' });
      });
    });
  }

  private async addToFavorite(translationId: string): Promise<void> {
    const response = await ApiService.addFavorite({ translation_id: translationId });
    if (response.success) {
      Toast.success('已添加到收藏');
    } else if (response.error?.code === 'CONFLICT') {
      Toast.info('已在收藏中');
    } else {
      Toast.error(response.error?.message || '添加失败');
    }
  }

  private async deleteHistoryItem(id: string): Promise<void> {
    if (!confirm('Are you sure you want to delete this translation?')) return;

    const response = await ApiService.deleteHistoryItem(id);
    if (response.success) {
      Toast.success('Deleted');
      this.loadHistory();
    } else {
      Toast.error('Delete failed');
    }
  }

  private escapeHtml(text: string): string {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
  }

  private formatDate(dateStr: string): string {
    const date = new Date(dateStr);
    return date.toLocaleString('en-US', {
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
    });
  }

  destroy(): void {
    this.particleBg?.destroy();
    this.header?.destroy();
    this.container.innerHTML = '';
  }
}
