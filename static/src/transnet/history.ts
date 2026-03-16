/**
 * Authenticated history page for past Transnet translations.
 */

import { PageShell } from '../shared/page-shell';
import { Toast } from '../shared/toast';
import { ApiService, HistoryItem, HistoryResponse } from './api';
import { t } from '../shared/language';

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
  private shell: PageShell | null = null;
  private historyData: HistoryResponse | null = null;
  private currentPage = 1;
  private filters = {
    source_lang: '',
    target_lang: '',
    input_type: '',
  };
  private mainElement: HTMLElement | null = null;

  constructor(container: HTMLElement = document.body) {
    this.container = container;
    this.container.innerHTML = '';
  }

  /**
   * Guard access, build the page shell, and load the first history page.
   */
  async render(): Promise<void> {
    this.shell = new PageShell(this.container, {
      requiresAuth: true,
      showFooter: false,
      showTransnetNav: true,
      mainClassName: 'profile-page',
    });

    this.mainElement = this.shell.mount();

    this.mainElement.innerHTML = `
      <div class="container">
        <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 32px;">
          <h1 style="font-size: 2rem; font-weight: 700;">${t('translationHistory')}</h1>
          <div style="display: flex; gap: 12px;">
            <select class="input-field filter-source" style="width: 140px; padding: 8px 12px;">
              <option value="">${t('historySource')}</option>
              ${LANGUAGES.map(lang => `<option value="${lang.code}">${lang.name}</option>`).join('')}
            </select>
            <select class="input-field filter-target" style="width: 140px; padding: 8px 12px;">
              <option value="">${t('historyTarget')}</option>
              ${LANGUAGES.map(lang => `<option value="${lang.code}">${lang.name}</option>`).join('')}
            </select>
            <select class="input-field filter-type" style="width: 120px; padding: 8px 12px;">
              <option value="">${t('historyType')}</option>
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

    this.bindFilterEvents();
    await this.loadHistory();
  }

  /**
   * Bind filter controls and reload history whenever a filter changes.
   */
  private bindFilterEvents(): void {
    const sourceFilter = this.mainElement?.querySelector('.filter-source');
    const targetFilter = this.mainElement?.querySelector('.filter-target');
    const typeFilter = this.mainElement?.querySelector('.filter-type');

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

  /**
   * Fetch the current history page using the active filters.
   */
  private async loadHistory(): Promise<void> {
    const response = await ApiService.getHistory({
      page: this.currentPage,
      limit: 20,
      ...this.filters,
      sort_by: 'created_at',
      sort_order: 'desc',
    });

    if (response.success && response.data) {
      this.historyData = response.data;
      this.renderHistoryList();
      this.renderPagination();
    } else {
      Toast.error(t('historyFailedToLoad'));
    }
  }

  /**
   * Render the list of history cards or the empty state.
   */
  private renderHistoryList(): void {
    const container = this.mainElement?.querySelector('.history-list');
    if (!container) return;

    const items = this.historyData?.translations || [];

    if (items.length === 0) {
      container.innerHTML = `
        <div class="empty-state">
          <div class="empty-state__icon">📜</div>
          <h3 class="empty-state__title">${t('historyNoHistoryYet')}</h3>
          <p class="empty-state__description">${t('historyStartTranslating')}</p>
        </div>
      `;
      return;
    }

    // Render split layout
    container.innerHTML = `
      <div class="history-split-layout">
        <div class="history-split-layout__list">
          ${items.map((item, index) => this.renderHistoryListItem(item, index)).join('')}
        </div>
        <div class="history-split-layout__details">
          ${items.length > 0 ? this.renderHistoryDetails(items[0]) : ''}
        </div>
      </div>
    `;

    this.bindListEvents();
  }

  /**
   * Render a single history list item.
   */
  private renderHistoryListItem(item: HistoryItem, index: number): string {
    const isActive = index === 0 ? 'history-split-layout__item--active' : '';
    return `
      <div class="history-split-layout__item ${isActive}" data-id="${item.translation_id}" data-index="${index}">
        <div class="history-split-layout__item-text truncated">${this.escapeHtml(item.text)}</div>
        <div class="history-split-layout__item-meta">
          <span class="history-card__badge">${item.source_lang.toUpperCase()} → ${item.target_lang.toUpperCase()}</span>
          <span>${item.input_type}</span>
        </div>
      </div>
    `;
  }

  /**
   * Render detailed view for a history item.
   */
  private renderHistoryDetails(item: HistoryItem): string {
    return `
      <div class="history-split-layout__details-header">
        <div style="font-size: 0.85rem; color: var(--text-tertiary);">${item.translation_id ? this.formatDate(new Date().toISOString()) : ''}</div>
        <div class="history-card__actions">
          <button class="btn btn--ghost btn--icon history-copy-btn" data-text="${this.escapeHtml(this.getTranslationText(item))}" title="${t('historyCopy')}">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
              <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
            </svg>
          </button>
          <button class="btn btn--ghost btn--icon history-favorite-btn" data-id="${item.translation_id}" title="${t('historyFavorite')}">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"></polygon>
            </svg>
          </button>
          <button class="btn btn--ghost btn--icon history-delete-btn" data-id="${item.translation_id}" title="${t('historyDelete')}">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <polyline points="3 6 5 6 21 6"></polyline>
              <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path>
            </svg>
          </button>
        </div>
      </div>
      <div class="history-split-layout__details-content">
        <div class="history-split-layout__details-source">${this.escapeHtml(item.text)}</div>
        <div class="history-split-layout__details-translation">${this.escapeHtml(this.getTranslationText(item))}</div>
      </div>
    `;
  }

  /**
   * Bind list item click and action events.
   */
  private bindListEvents(): void {
    // List item clicks to update details
    this.mainElement?.querySelectorAll('.history-split-layout__item').forEach((item) => {
      item.addEventListener('click', () => {
        const index = parseInt((item as HTMLElement).dataset.index || '0');
        const items = this.historyData?.translations || [];
        if (items[index]) {
          this.updateDetails(items[index], index);
        }
      });
    });

    // Copy button
    this.mainElement?.querySelectorAll('.history-copy-btn').forEach((btn) => {
      btn.addEventListener('click', (e) => {
        e.stopPropagation();
        const text = btn.getAttribute('data-text');
        if (text) {
          navigator.clipboard.writeText(text);
          Toast.success(t('historyCopied'));
        }
      });
    });

    // Favorite button
    this.mainElement?.querySelectorAll('.history-favorite-btn').forEach((btn) => {
      btn.addEventListener('click', (e) => {
        e.stopPropagation();
        const id = btn.getAttribute('data-id');
        if (id) this.addToFavorite(id);
      });
    });

    // Delete button
    this.mainElement?.querySelectorAll('.history-delete-btn').forEach((btn) => {
      btn.addEventListener('click', (e) => {
        e.stopPropagation();
        const id = btn.getAttribute('data-id');
        if (id) this.deleteHistoryItem(id);
      });
    });
  }

  /**
   * Update details panel when a list item is clicked.
   */
  private updateDetails(item: HistoryItem, index: number): void {
    const items = this.mainElement?.querySelectorAll('.history-split-layout__item');
    const details = this.mainElement?.querySelector('.history-split-layout__details');

    if (items) {
      items.forEach((itemEl, i) => {
        if (i === index) {
          itemEl.classList.add('history-split-layout__item--active');
        } else {
          itemEl.classList.remove('history-split-layout__item--active');
        }
      });
    }

    if (details && item) {
      details.innerHTML = this.renderHistoryDetails(item);
      this.bindListEvents();
    }
  }

  /**
   * Render pagination controls for the current response set.
   */
  private renderPagination(): void {
    const container = this.mainElement?.querySelector('.pagination');
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

  /**
   * Save a history item to the favorites list.
   */
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

  /**
   * Extract translation text from translation object (handles type-specific structures)
   */
  private getTranslationText(item: HistoryItem): string {
    if (typeof item.translation === 'string') {
      return item.translation;
    }
    if (item.translation && typeof item.translation === 'object') {
      // Try common fields that might contain the translated text
      if ('translation' in item.translation && typeof item.translation.translation === 'string') {
        return item.translation.translation;
      }
      if ('translations' in item.translation && Array.isArray(item.translation.translations) && item.translation.translations.length > 0) {
        return item.translation.translations[0];
      }
      // Fallback: try to stringify or return empty
      return JSON.stringify(item.translation);
    }
    return '';
  }

  /**
   * Remove a single history entry after user confirmation.
   */
  private async deleteHistoryItem(translation_id: string): Promise<void> {
    if (!confirm(t('historyDeleteConfirm'))) return;

    const response = await ApiService.deleteHistoryItem(translation_id);
    if (response.success) {
      Toast.success(t('historyDeleted'));
      this.loadHistory();
    } else {
      Toast.error(t('historyDeleteFailed'));
    }
  }

  /**
   * Escape text before inserting it into history-card HTML.
   */
  private escapeHtml(text: string): string {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
  }

  /**
   * Format API timestamps for display in the history list.
   */
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

  /**
   * Remove the page and its shared widgets from the DOM.
   */
  destroy(): void {
    this.shell?.destroy();
    this.shell = null;
    this.mainElement = null;
  }
}
