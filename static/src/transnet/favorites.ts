/**
 * Authenticated favorites page for saved Transnet translations.
 */

import { PageShell } from '../shared/page-shell';
import { Toast } from '../shared/toast';
import { ApiService, Favorite, FavoritesResponse } from '../services/tran_api/api';

export class Favorites {
  private container: HTMLElement;
  private shell: PageShell | null = null;
  private favoritesData: FavoritesResponse | null = null;
  private currentPage = 1;
  private mainElement: HTMLElement | null = null;

  constructor(container: HTMLElement = document.body) {
    this.container = container;
    this.container.innerHTML = '';
  }

  /**
   * Guard access, build the page shell, and load the first favorites page.
   */
  async render(): Promise<void> {
    this.shell = new PageShell(this.container, {
      requiresAuth: true,
      showFooter: false,
      mainClassName: 'profile-page',
    });

    this.mainElement = this.shell.mount();

    this.mainElement.innerHTML = `
      <div class="container">
        <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 32px;">
          <h1 style="font-size: 2rem; font-weight: 700;">My Favorites</h1>
          <div style="font-size: 0.9rem; color: var(--text-secondary);">
            <span class="favorites-count">0</span> items
          </div>
        </div>

        <div class="favorites-grid" style="display: grid; gap: 20px;"></div>

        <div class="pagination"></div>
      </div>
    `;

    await this.loadFavorites();
  }

  /**
   * Fetch the current favorites page from the API.
   */
  private async loadFavorites(): Promise<void> {
    const response = await ApiService.getFavorites({
      page: this.currentPage,
      limit: 10,
      sort_by: 'created_at',
      sort_order: 'desc',
    });

    if (response.success && response.data) {
      this.favoritesData = response.data;
      this.updateCount();
      this.renderFavorites();
      this.renderPagination();
    } else {
      Toast.error('Failed to load favorites');
    }
  }

  /**
   * Update the visible favorites count from the latest pagination data.
   */
  private updateCount(): void {
    const countEl = this.mainElement?.querySelector('.favorites-count');
    if (countEl && this.favoritesData) {
      countEl.textContent = this.favoritesData.pagination.total.toString();
    }
  }

  /**
   * Render all favorite cards or the empty state for the route.
   */
  private renderFavorites(): void {
    const container = this.mainElement?.querySelector('.favorites-grid');
    if (!container) return;

    const items = this.favoritesData?.favorites || [];

    if (items.length === 0) {
      container.innerHTML = `
        <div class="empty-state">
          <div class="empty-state__icon">⭐</div>
          <h3 class="empty-state__title">No Favorites Yet</h3>
          <p class="empty-state__description">Click the star icon next to translation results to add them to favorites</p>
        </div>
      `;
      return;
    }

    container.innerHTML = items.map((item, index) => this.renderFavoriteCard(item, index)).join('');

    this.bindCardEvents();
  }

  /**
   * Render one favorite translation card with note-edit controls.
   */
  private renderFavoriteCard(item: Favorite, index: number): string {
    const translation = item.translation;
    const wordMeaning = item.word_meaning;

    if (!translation) return '';

    return `
      <div class="glass-card favorite-card animate-fade-in-up" style="
        animation-delay: ${index * 0.05}s;
        padding: 24px;
        position: relative;
      " data-id="${item.id}">
        <div style="display: flex; justify-content: space-between; align-items: flex-start; margin-bottom: 16px;">
          <div style="display: flex; align-items: center; gap: 8px;">
            <span style="font-size: 1.5rem;">⭐</span>
            <span class="history-card__badge">${translation.source_lang.toUpperCase()} → ${translation.target_lang.toUpperCase()}</span>
            <span style="font-size: 0.8rem; color: var(--text-tertiary);">${this.formatDate(item.created_at)}</span>
          </div>
          <div style="display: flex; gap: 8px;">
            <button class="btn btn--ghost btn--icon favorite-copy-btn" data-text="${this.escapeHtml(translation.translation)}" title="复制">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
                <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
              </svg>
            </button>
            <button class="btn btn--ghost btn--icon favorite-edit-btn" data-id="${item.id}" title="编辑备注">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"></path>
                <path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"></path>
              </svg>
            </button>
            <button class="btn btn--ghost btn--icon favorite-delete-btn" data-id="${item.id}" title="删除">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <polyline points="3 6 5 6 21 6"></polyline>
                <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path>
              </svg>
            </button>
          </div>
        </div>

        <div style="margin-bottom: 16px;">
          <div style="font-size: 0.9rem; color: var(--text-secondary); margin-bottom: 4px;">${this.escapeHtml(translation.text)}</div>
          <div style="font-size: 1.1rem; color: var(--accent-primary); font-weight: 500;">${this.escapeHtml(translation.translation)}</div>
        </div>

        ${wordMeaning ? `
          <div style="
            background: rgba(0, 240, 255, 0.05);
            border: 1px solid rgba(0, 240, 255, 0.1);
            border-radius: 12px;
            padding: 16px;
            margin-bottom: 16px;
          ">
            <div style="display: flex; align-items: center; gap: 8px; margin-bottom: 8px;">
              <span style="font-weight: 600;">${wordMeaning.word}</span>
              <span class="history-card__badge">${wordMeaning.pos}</span>
            </div>
            <div style="font-size: 0.9rem; color: var(--text-secondary); margin-bottom: 8px;">${wordMeaning.meaning}</div>
            ${wordMeaning.related_words.length > 0 ? `
              <div style="display: flex; flex-wrap: wrap; gap: 8px;">
                ${wordMeaning.related_words.map(word => `
                  <span style="font-size: 0.8rem; color: var(--accent-secondary); background: rgba(184, 41, 247, 0.1); padding: 2px 8px; border-radius: 4px;">${word}</span>
                  `).join('')}
              </div>
            ` : ''}
          </div>
        ` : ''}

        <div class="note-section" data-id="${item.id}">
          ${item.note ? `
            <div class="note-display" style="font-size: 0.9rem; color: var(--text-tertiary); padding: 8px; background: rgba(255,255,255,0.03); border-radius: 8px;">
              <span style="color: var(--text-secondary);">Note:</span> ${this.escapeHtml(item.note)}
            </div>
          ` : '<div class="note-display" style="font-size: 0.85rem; color: var(--text-tertiary); font-style: italic;">Click edit to add a note...</div>'}

          <div class="note-edit hidden" style="display: flex; gap: 8px; margin-top: 8px;">
            <input type="text" class="input-field note-input" value="${this.escapeHtml(item.note || '')}" placeholder="Add a note..." style="flex: 1; padding: 8px 12px;">
            <button class="btn btn--primary note-save" style="padding: 8px 16px;">Save</button>
            <button class="btn btn--ghost note-cancel" style="padding: 8px 16px;">Cancel</button>
          </div>
        </div>
      </div>
    `;
  }

  /**
   * Bind card-level actions such as copy, edit, save, and delete.
   */
  private bindCardEvents(): void {
    this.mainElement?.querySelectorAll('.favorite-copy-btn').forEach(btn => {
      btn.addEventListener('click', () => {
        const text = btn.getAttribute('data-text');
        if (text) {
          navigator.clipboard.writeText(text);
          Toast.success('Copied');
        }
      });
    });

    this.mainElement?.querySelectorAll('.favorite-edit-btn').forEach(btn => {
      btn.addEventListener('click', () => {
        const id = btn.getAttribute('data-id');
        if (id) this.startEditing(id);
      });
    });

    this.mainElement?.querySelectorAll('.favorite-delete-btn').forEach(btn => {
      btn.addEventListener('click', () => {
        const id = btn.getAttribute('data-id');
        if (id) this.deleteFavorite(id);
      });
    });

    this.mainElement?.querySelectorAll('.note-save').forEach(btn => {
      btn.addEventListener('click', () => {
        const card = btn.closest('.favorite-card');
        const id = card?.getAttribute('data-id');
        const input = card?.querySelector('.note-input') as HTMLInputElement;
        if (id && input) {
          this.saveNote(id, input.value);
        }
      });
    });

    this.mainElement?.querySelectorAll('.note-cancel').forEach(btn => {
      btn.addEventListener('click', () => {
        this.stopEditing();
      });
    });
  }

  /**
   * Show the inline note editor for the chosen favorite card.
   */
  private startEditing(id: string): void {
    this.mainElement?.querySelectorAll('.note-section').forEach(section => {
      section.querySelector('.note-display')?.classList.remove('hidden');
      section.querySelector('.note-edit')?.classList.add('hidden');
    });

    const card = this.mainElement?.querySelector(`.favorite-card[data-id="${id}"]`);
    if (card) {
      card.querySelector('.note-display')?.classList.add('hidden');
      card.querySelector('.note-edit')?.classList.remove('hidden');
      const input = card.querySelector('.note-input') as HTMLInputElement;
      input?.focus();
    }
  }

  /**
   * Restore the non-editing view for all note editors.
   */
  private stopEditing(): void {
    this.renderFavorites();
  }

  /**
   * Persist the edited note for a single favorite.
   */
  private async saveNote(id: string, note: string): Promise<void> {
    const response = await ApiService.updateFavorite(id, note);
    if (response.success) {
      Toast.success('备注已更新');
      this.loadFavorites();
    } else {
      Toast.error('更新失败');
    }
  }

  /**
   * Delete a favorite entry after user confirmation.
   */
  private async deleteFavorite(id: string): Promise<void> {
    if (!confirm('Are you sure you want to delete this favorite?')) return;

    const response = await ApiService.deleteFavorite(id);
    if (response.success) {
      Toast.success('Deleted');
      this.loadFavorites();
    } else {
      Toast.error('Delete failed');
    }
  }

  /**
   * Render pagination controls for the current favorites response.
   */
  private renderPagination(): void {
    const container = this.mainElement?.querySelector('.pagination');
    if (!container || !this.favoritesData) return;

    const { page, total_pages } = this.favoritesData.pagination;

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
        this.loadFavorites();
        window.scrollTo({ top: 0, behavior: 'smooth' });
      });
    });
  }

  /**
   * Escape user-provided text before inserting it into generated markup.
   */
  private escapeHtml(text: string): string {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
  }

  /**
   * Format API dates for display on the favorites cards.
   */
  private formatDate(dateStr: string): string {
    const date = new Date(dateStr);
    return date.toLocaleDateString('zh-CN');
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
