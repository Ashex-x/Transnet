/**
 * Settings page - demonstrates shared shell with simple content.
 */

import { PageShell } from '../shared/page-shell';
import { t } from '../shared/language';

export class Setting {
  private container: HTMLElement;
  private shell: PageShell | null = null;

  constructor(container: HTMLElement = document.body) {
    this.container = container;
    this.container.innerHTML = '';
  }

  /**
   * Render settings page content.
   */
  async render(): Promise<void> {
    this.shell = new PageShell(this.container, {
      requiresAuth: false,
      showFooter: true,
      mainClassName: 'setting-page',
    });

    const main = this.shell.mount();

    main.innerHTML = `
      <div class="container">
        <div class="glass-card" style="max-width: 800px; margin: 0 auto; padding: 40px;">
          <h1 style="font-size: 2.5rem; font-weight: 700; margin-bottom: 24px;">${t('settings')}</h1>
          <p style="color: var(--text-secondary); margin-bottom: 40px;">
            ${t('settingsDescription')}
          </p>

          <div style="display: flex; flex-direction: column; gap: 32px;">
            <div>
              <h3 style="font-size: 1.25rem; font-weight: 600; margin-bottom: 16px;">${t('appearance')}</h3>
              <div style="display: flex; align-items: center; justify-content: space-between; padding: 16px 0; border-bottom: 1px solid rgba(255,255,255,0.05);">
                <div>
                  <div style="font-weight: 500; margin-bottom: 4px;">${t('darkMode')}</div>
                  <div style="font-size: 0.9rem; color: var(--text-tertiary);">${t('darkModeFixed')}</div>
                </div>
                <div style="width: 48px; height: 24px; background: var(--accent-primary); border-radius: 12px; position: relative; cursor: not-allowed;">
                  <div style="width: 20px; height: 20px; background: white; border-radius: 50%; position: absolute; right: 2px; top: 2px;"></div>
                </div>
              </div>
            </div>

            <div>
              <h3 style="font-size: 1.25rem; font-weight: 600; margin-bottom: 16px;">${t('languageSettings')}</h3>
              <div style="display: flex; align-items: center; justify-content: space-between; padding: 16px 0; border-bottom: 1px solid rgba(255,255,255,0.05);">
                <div>
                  <div style="font-weight: 500; margin-bottom: 4px;">${t('interfaceLanguage')}</div>
                  <div style="font-size: 0.9rem; color: var(--text-tertiary);">${t('languageSetViaHome')}</div>
                </div>
                <button style="padding: 8px 16px; background: rgba(0,240,255,0.1); color: var(--accent-primary); border: 1px solid var(--accent-primary); border-radius: 8px; cursor: not-allowed;" disabled>${t('configure')}</button>
              </div>
            </div>

            <div>
              <h3 style="font-size: 1.25rem; font-weight: 600; margin-bottom: 16px;">${t('translationSettings')}</h3>
              <div style="display: flex; flex-direction: column; gap: 16px;">
                <div style="display: flex; align-items: center; justify-content: space-between; padding: 16px 0; border-bottom: 1px solid rgba(255,255,255,0.05);">
                  <div>
                    <div style="font-weight: 500; margin-bottom: 4px;">${t('autoDetectInput')}</div>
                    <div style="font-size: 0.9rem; color: var(--text-tertiary);">${t('smartDetectionEnabled')}</div>
                  </div>
                  <div style="width: 48px; height: 24px; background: var(--success); border-radius: 12px; position: relative; cursor: not-allowed;">
                    <div style="width: 20px; height: 20px; background: white; border-radius: 50%; position: absolute; right: 2px; top: 2px;"></div>
                  </div>
                </div>
                <div style="display: flex; align-items: center; justify-content: space-between; padding: 16px 0; border-bottom: 1px solid rgba(255,255,255,0.05);">
                  <div>
                    <div style="font-weight: 500; margin-bottom: 4px;">${t('saveToHistory')}</div>
                    <div style="font-size: 0.9rem; color: var(--text-tertiary);">${t('autoSaveTranslations')}</div>
                  </div>
                  <div style="width: 48px; height: 24px; background: var(--success); border-radius: 12px; position: relative; cursor: not-allowed;">
                    <div style="width: 20px; height: 20px; background: white; border-radius: 50%; position: absolute; right: 2px; top: 2px;"></div>
                  </div>
                </div>
              </div>
            </div>

            <div>
              <h3 style="font-size: 1.25rem; font-weight: 600; margin-bottom: 16px;">${t('privacy')}</h3>
              <div style="display: flex; flex-direction: column; gap: 16px;">
                <div style="display: flex; align-items: center; justify-content: space-between; padding: 16px 0; border-bottom: 1px solid rgba(255,255,255,0.05);">
                  <div>
                    <div style="font-weight: 500; margin-bottom: 4px;">${t('dataStorage')}</div>
                    <div style="font-size: 0.9rem; color: var(--text-tertiary);">${t('dataStoredSecurely')}</div>
                  </div>
                  <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="var(--success)" stroke-width="2">
                    <path d="M20 6L9 17l-5-5"></path>
                    <circle cx="12" cy="12" r="10"></circle>
                  </svg>
                </div>
                <div style="display: flex; align-items: center; justify-content: space-between; padding: 16px 0; border-bottom: 1px solid rgba(255,255,255,0.05);">
                  <div>
                    <div style="font-weight: 500; margin-bottom: 4px;">${t('accountDeletion')}</div>
                    <div style="font-size: 0.9rem; color: var(--text-tertiary);">${t('removeDataPermanently')}</div>
                  </div>
                  <button style="padding: 8px 16px; background: rgba(255,99,71,0.1); color: var(--error); border: 1px solid var(--error); border-radius: 8px; cursor: not-allowed;" disabled>${t('requestDeletion')}</button>
                </div>
              </div>
            </div>
          </div>

          <div style="margin-top: 40px; padding: 20px; background: rgba(184, 41, 247, 0.05); border: 1px solid rgba(184, 41, 247, 0.2); border-radius: 12px;">
            <p style="font-size: 0.9rem; color: var(--text-secondary); margin: 0;">
              <strong>${t('settingsComingSoon')}</strong> ${t('settingsComingSoonDesc')}
            </p>
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
