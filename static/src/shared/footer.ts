/**
 * Shared footer component for application pages.
 *
 * Provides a consistent footer with configurable content.
 */

import { t } from './language';

export interface FooterOptions {
  /**
   * Custom content for the footer.
   * If not provided, uses default content.
   */
  content?: string;

  /**
   * Additional CSS class to apply to the footer.
   */
  className?: string;
}

export class Footer {
  private element: HTMLElement;

  constructor(options: FooterOptions = {}) {
    this.element = document.createElement('footer');
    this.element.className = 'page-footer' + (options.className ? ` ${options.className}` : '');

    const content = options.content || `
      <span>${t('currentTheme')}</span>
      <span>${t('backgroundPreserved')}</span>
    `;
    this.element.innerHTML = `
      <div class="container">
        ${content}
      </div>
    `;
  }

  /**
   * Mount the footer into a parent element.
   */
  mount(parent: HTMLElement): void {
    parent.appendChild(this.element);
  }

  /**
   * Remove the footer from the DOM.
   */
  destroy(): void {
    this.element.remove();
  }

  /**
   * Get the footer element for direct manipulation if needed.
   */
  getElement(): HTMLElement {
    return this.element;
  }
}
