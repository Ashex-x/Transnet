/**
 * About page - demonstrates shared shell with simple content.
 */

import { PageShell } from '../shared/page-shell';

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
          <h1 style="font-size: 2.5rem; font-weight: 700; margin-bottom: 24px;">About Transnet</h1>
          <p style="font-size: 1.1rem; color: var(--text-secondary); line-height: 1.7; margin-bottom: 32px;">
            Transnet is an AI-powered intelligent translation platform designed to bridge language barriers across digital worlds. Our mission is to make seamless communication accessible to everyone.
          </p>

          <div style="display: grid; gap: 24px; grid-template-columns: repeat(auto-fit, minmax(250px, 1fr)); margin-bottom: 32px;">
            <div>
              <h3 style="font-size: 1.25rem; font-weight: 600; margin-bottom: 12px;">Multi-Language Support</h3>
              <p style="color: var(--text-secondary); line-height: 1.6;">
                Translate between English, Spanish, French, German, Chinese, Japanese, and Korean with advanced AI models.
              </p>
            </div>
            <div>
              <h3 style="font-size: 1.25rem; font-weight: 600; margin-bottom: 12px;">Smart Translation</h3>
              <p style="color: var(--text-secondary); line-height: 1.6;">
                Automatically detects input types and provides context-aware translations for words, phrases, sentences, and full text.
              </p>
            </div>
            <div>
              <h3 style="font-size: 1.25rem; font-weight: 600; margin-bottom: 12px;">Translation History</h3>
              <p style="color: var(--text-secondary); line-height: 1.6;">
                Keep track of all your translations with powerful search and filter capabilities to find past work quickly.
              </p>
            </div>
            <div>
              <h3 style="font-size: 1.25rem; font-weight: 600; margin-bottom: 12px;">Favorites</h3>
              <p style="color: var(--text-secondary); line-height: 1.6;">
                Save your best translations with custom notes and enhanced word meanings for easy reference later.
              </p>
            </div>
          </div>

          <div style="text-align: center; padding-top: 32px; border-top: 1px solid rgba(255,255,255,0.1);">
            <p style="color: var(--text-tertiary);">Built with cutting-edge AI technology</p>
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
