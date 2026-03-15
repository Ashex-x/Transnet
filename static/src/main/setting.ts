/**
 * Settings page - demonstrates shared shell with simple content.
 */

import { PageShell } from '../shared/page-shell';

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
          <h1 style="font-size: 2.5rem; font-weight: 700; margin-bottom: 24px;">Settings</h1>
          <p style="color: var(--text-secondary); margin-bottom: 40px;">
            Customize your Transnet experience with these settings.
          </p>

          <div style="display: flex; flex-direction: column; gap: 32px;">
            <div>
              <h3 style="font-size: 1.25rem; font-weight: 600; margin-bottom: 16px;">Appearance</h3>
              <div style="display: flex; align-items: center; justify-content: space-between; padding: 16px 0; border-bottom: 1px solid rgba(255,255,255,0.05);">
                <div>
                  <div style="font-weight: 500; margin-bottom: 4px;">Dark Mode</div>
                  <div style="font-size: 0.9rem; color: var(--text-tertiary);">Currently fixed to dark theme</div>
                </div>
                <div style="width: 48px; height: 24px; background: var(--accent-primary); border-radius: 12px; position: relative; cursor: not-allowed;">
                  <div style="width: 20px; height: 20px; background: white; border-radius: 50%; position: absolute; right: 2px; top: 2px;"></div>
                </div>
              </div>
            </div>

            <div>
              <h3 style="font-size: 1.25rem; font-weight: 600; margin-bottom: 16px;">Language</h3>
              <div style="display: flex; align-items: center; justify-content: space-between; padding: 16px 0; border-bottom: 1px solid rgba(255,255,255,0.05);">
                <div>
                  <div style="font-weight: 500; margin-bottom: 4px;">Interface Language</div>
                  <div style="font-size: 0.9rem; color: var(--text-tertiary);">Set via home page language selector</div>
                </div>
                <button style="padding: 8px 16px; background: rgba(0,240,255,0.1); color: var(--accent-primary); border: 1px solid var(--accent-primary); border-radius: 8px; cursor: not-allowed;" disabled>Configure</button>
              </div>
            </div>

            <div>
              <h3 style="font-size: 1.25rem; font-weight: 600; margin-bottom: 16px;">Translation</h3>
              <div style="display: flex; flex-direction: column; gap: 16px;">
                <div style="display: flex; align-items: center; justify-content: space-between; padding: 16px 0; border-bottom: 1px solid rgba(255,255,255,0.05);">
                  <div>
                    <div style="font-weight: 500; margin-bottom: 4px;">Auto-detect Input Type</div>
                    <div style="font-size: 0.9rem; color: var(--text-tertiary);">Smart detection enabled by default</div>
                  </div>
                  <div style="width: 48px; height: 24px; background: var(--success); border-radius: 12px; position: relative; cursor: not-allowed;">
                    <div style="width: 20px; height: 20px; background: white; border-radius: 50%; position: absolute; right: 2px; top: 2px;"></div>
                  </div>
                </div>
                <div style="display: flex; align-items: center; justify-content: space-between; padding: 16px 0; border-bottom: 1px solid rgba(255,255,255,0.05);">
                  <div>
                    <div style="font-weight: 500; margin-bottom: 4px;">Save to History</div>
                    <div style="font-size: 0.9rem; color: var(--text-tertiary);">Automatically save translations</div>
                  </div>
                  <div style="width: 48px; height: 24px; background: var(--success); border-radius: 12px; position: relative; cursor: not-allowed;">
                    <div style="width: 20px; height: 20px; background: white; border-radius: 50%; position: absolute; right: 2px; top: 2px;"></div>
                  </div>
                </div>
              </div>
            </div>

            <div>
              <h3 style="font-size: 1.25rem; font-weight: 600; margin-bottom: 16px;">Privacy</h3>
              <div style="display: flex; flex-direction: column; gap: 16px;">
                <div style="display: flex; align-items: center; justify-content: space-between; padding: 16px 0; border-bottom: 1px solid rgba(255,255,255,0.05);">
                  <div>
                    <div style="font-weight: 500; margin-bottom: 4px;">Data Storage</div>
                    <div style="font-size: 0.9rem; color: var(--text-tertiary);">Your translations are stored securely</div>
                  </div>
                  <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="var(--success)" stroke-width="2">
                    <path d="M20 6L9 17l-5-5"></path>
                    <circle cx="12" cy="12" r="10"></circle>
                  </svg>
                </div>
                <div style="display: flex; align-items: center; justify-content: space-between; padding: 16px 0; border-bottom: 1px solid rgba(255,255,255,0.05);">
                  <div>
                    <div style="font-weight: 500; margin-bottom: 4px;">Account Deletion</div>
                    <div style="font-size: 0.9rem; color: var(--text-tertiary);">Remove all your data permanently</div>
                  </div>
                  <button style="padding: 8px 16px; background: rgba(255,99,71,0.1); color: var(--error); border: 1px solid var(--error); border-radius: 8px; cursor: not-allowed;" disabled>Request Deletion</button>
                </div>
              </div>
            </div>
          </div>

          <div style="margin-top: 40px; padding: 20px; background: rgba(184, 41, 247, 0.05); border: 1px solid rgba(184, 41, 247, 0.2); border-radius: 12px;">
            <p style="font-size: 0.9rem; color: var(--text-secondary); margin: 0;">
              <strong>Settings Coming Soon:</strong> More customization options will be available in future updates. Stay tuned for enhanced personalization features.
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
