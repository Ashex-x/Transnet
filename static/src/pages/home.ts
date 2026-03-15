/**
 * Home Page
 * Landing page with hero section and translator
 */

import { ParticleBackground } from '../components/ParticleBackground';
import { Header } from '../components/Header';
import { Translator } from '../components/Translator';
import { ApiService, HistoryItem } from '../services/api';
import { AuthService } from '../services/auth';
import { router } from '../router';

export class Home {
  private container: HTMLElement;
  private particleBg: ParticleBackground | null = null;
  private header: Header | null = null;
  private translator: Translator | null = null;
  private recentHistory: HistoryItem[] = [];

  constructor(container: HTMLElement = document.body) {
    this.container = container;
    this.container.innerHTML = '';
  }

  async render(): Promise<void> {
    // Add particle background
    this.particleBg = new ParticleBackground(this.container);

    // Add header
    this.header = new Header();
    this.header.mount(this.container);

    // Create main content
    const main = document.createElement('main');
    main.className = 'hero';
    main.innerHTML = `
      <h1 class="hero__title animate-fade-in">Transnet</h1>
      <p class="hero__subtitle animate-fade-in stagger-1">
        AI-powered translation connecting every language in the world
      </p>
      <div class="hero__translator animate-fade-in-up stagger-2"></div>
      <div class="hero__recent animate-fade-in-up stagger-3"></div>
    `;

    this.container.appendChild(main);

    // Mount translator
    const translatorContainer = main.querySelector('.hero__translator');
    if (translatorContainer) {
      this.translator = new Translator();
      this.translator.mount(translatorContainer as HTMLElement);
    }

    // Load recent history if authenticated
    if (AuthService.isAuthenticated()) {
      await this.loadRecentHistory();
      this.renderRecentHistory();
    }

    // Add footer
    this.addFooter();
  }

  private async loadRecentHistory(): Promise<void> {
    const response = await ApiService.getHistory({ limit: 3 });
    if (response.success && response.data) {
      this.recentHistory = response.data.translations;
    }
  }

  private renderRecentHistory(): void {
    const container = this.container.querySelector('.hero__recent');
    if (!container || this.recentHistory.length === 0) return;

    container.innerHTML = `
      <h2 class="hero__section-title">Recent Translations</h2>
      <div class="recent-history">
        ${this.recentHistory.map(item => `
          <div class="glass-card history-card" style="margin-bottom: 12px; cursor: pointer;" data-id="${item.id}">
            <div class="history-card__content">
              <div class="history-card__text">${this.escapeHtml(item.text.substring(0, 50))}${item.text.length > 50 ? '...' : ''}</div>
              <div class="history-card__translation">${this.escapeHtml(item.translation.substring(0, 50))}${item.translation.length > 50 ? '...' : ''}</div>
              <div class="history-card__meta">
                <span class="history-card__badge">${item.source_lang} → ${item.target_lang}</span>
                <span>${this.formatDate(item.created_at)}</span>
              </div>
            </div>
          </div>
        `).join('')}
      </div>
    `;

    // Bind click events
    container.querySelectorAll('.history-card').forEach(card => {
      card.addEventListener('click', () => {
        const id = card.getAttribute('data-id');
        if (id) {
          router.navigate(`/history?id=${id}`);
        }
      });
    });
  }

  private addFooter(): void {
    const footer = document.createElement('footer');
    footer.style.cssText = `
      text-align: center;
      padding: 48px 24px;
      color: rgba(255, 255, 255, 0.4);
      font-size: 0.875rem;
    `;
    footer.innerHTML = `
      <p>© 2026 Transnet. All rights reserved.</p>
      <p style="margin-top: 8px;">Powered by AI</p>
    `;
    this.container.appendChild(footer);
  }

  private escapeHtml(text: string): string {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
  }

  private formatDate(dateStr: string): string {
    const date = new Date(dateStr);
    const now = new Date();
    const diff = now.getTime() - date.getTime();
    
    const minutes = Math.floor(diff / 60000);
    const hours = Math.floor(diff / 3600000);
    const days = Math.floor(diff / 86400000);
    
    if (minutes < 1) return 'Just now';
    if (minutes < 60) return `${minutes}m ago`;
    if (hours < 24) return `${hours}h ago`;
    if (days < 7) return `${days}d ago`;
    
    return date.toLocaleDateString('en-US');
  }

  destroy(): void {
    this.particleBg?.destroy();
    this.header?.destroy();
    this.translator?.destroy();
    this.container.innerHTML = '';
  }
}
