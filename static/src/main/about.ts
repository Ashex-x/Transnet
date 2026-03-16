/**
 * About page - demonstrates shared shell with simple content.
 */

import { PageShell } from '../shared/page-shell';
import { t } from '../shared/language';

export class About {
  private container: HTMLElement;
  private shell: PageShell | null = null;

  constructor(container: HTMLElement = document.body) {
    this.container = container;
    this.container.innerHTML = '';
  }

  /**
   * Render about page content.
   */
  async render(): Promise<void> {
    this.shell = new PageShell(this.container, {
      requiresAuth: false,
      showFooter: true,
      mainClassName: 'about-page',
    });

    const main = this.shell.mount();

    main.innerHTML = `
      <div class="container">
        <div class="glass-card" style="max-width: 800px; margin: 0 auto; padding: 40px;">
          <h1 style="font-size: 2.5rem; font-weight: 700; margin-bottom: 24px;">${t('aboutTransnet')}</h1>
          <p style="font-size: 1.1rem; color: var(--text-secondary); line-height: 1.7; margin-bottom: 32px;">
            ${t('aboutDescription')}
          </p>

          <div style="display: grid; gap: 24px; grid-template-columns: repeat(auto-fit, minmax(250px, 1fr)); margin-bottom: 32px;">
            <div>
              <h3 style="font-size: 1.25rem; font-weight: 600; margin-bottom: 12px;">${t('multiLanguageSupport')}</h3>
              <p style="color: var(--text-secondary); line-height: 1.6;">
                ${t('multiLanguageDesc')}
              </p>
            </div>
            <div>
              <h3 style="font-size: 1.25rem; font-weight: 600; margin-bottom: 12px;">${t('smartTranslation')}</h3>
              <p style="color: var(--text-secondary); line-height: 1.6;">
                ${t('smartTranslationDesc')}
              </p>
            </div>
            <div>
              <h3 style="font-size: 1.25rem; font-weight: 600; margin-bottom: 12px;">${t('translationHistory')}</h3>
              <p style="color: var(--text-secondary); line-height: 1.6;">
                ${t('translationHistoryDesc')}
              </p>
            </div>
            <div>
              <h3 style="font-size: 1.25rem; font-weight: 600; margin-bottom: 12px;">${t('favorites')}</h3>
              <p style="color: var(--text-secondary); line-height: 1.6;">
                ${t('favoritesDesc')}
              </p>
            </div>
          </div>

          <div style="text-align: center; padding-top: 32px; border-top: 1px solid rgba(255,255,255,0.1);">
            <p style="color: var(--text-tertiary);">${t('builtWithAI')}</p>
          </div>
        </div>
      </div>
    `;
  }

  /**
   * Remove page and shell from DOM.
   */
  destroy(): void {
    this.shell?.destroy();
    this.shell = null;
  }
}
