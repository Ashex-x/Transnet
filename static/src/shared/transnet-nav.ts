/**
 * Secondary navigation bar for Transnet pages.
 *
 * Features typing animation that reveals full text on hover,
 * with support for both English and Chinese.
 */

import { router } from '../router';
import { t, onLanguageChange } from './language';

interface TransnetNavItem {
  route: string;
  fallbackLetter: string;
  labelKey: 'navTranslate' | 'navHistory' | 'navFavorites';
}

export class TransnetNav {
  private element: HTMLElement;
  private unsubscribeLanguage: (() => void) | null = null;
  private readonly navItems: TransnetNavItem[] = [
    {
      route: '/transnet',
      fallbackLetter: 'T',
      labelKey: 'navTranslate',
    },
    {
      route: '/transnet/history',
      fallbackLetter: 'H',
      labelKey: 'navHistory',
    },
    {
      route: '/transnet/favorites',
      fallbackLetter: 'F',
      labelKey: 'navFavorites',
    },
  ];

  constructor() {
    this.element = document.createElement('nav');
    this.element.className = 'transnet-nav';
    this.render();

    this.unsubscribeLanguage = onLanguageChange(() => {
      this.updateButtonTitles();
    });
  }

  /**
   * Render the navigation bar with three route buttons.
   */
  private render(): void {
    const buttons = this.navItems.map((item) => {
      const activeClass = this.isActive(item.route) ? 'transnet-nav__btn--active' : '';
      const currentPage = activeClass ? 'page' : 'false';
      const { firstCharacter, remainingText } = this.getLabelParts(item.labelKey, item.fallbackLetter);

      return `
        <button
          type="button"
          class="transnet-nav__btn ${activeClass}"
          data-route="${item.route}"
          data-label-key="${item.labelKey}"
          aria-current="${currentPage}"
        >
          <span class="transnet-nav__btn-letter" aria-hidden="true">${firstCharacter}</span>
          <span class="transnet-nav__btn-full">${remainingText}</span>
        </button>
      `;
    }).join('');

    this.element.innerHTML = `
      <div class="transnet-nav__container">
        ${buttons}
      </div>
    `;

    this.bindEvents();
  }

  /**
   * Check if a route is currently active.
   */
  private isActive(path: string): boolean {
    const currentPath = this.normalizePath(window.location.pathname);
    if (path === '/transnet') {
      return currentPath === '/transnet';
    }

    return currentPath === path || currentPath.startsWith(`${path}/`);
  }

  /**
   * Normalise the browser pathname so route comparisons stay consistent.
   */
  private normalizePath(path: string): string {
    if (path.length > 1 && path.endsWith('/')) {
      return path.slice(0, -1);
    }

    return path;
  }

  /**
   * Split a translated label into the leading character and the remainder.
   */
  private getLabelParts(
    labelKey: TransnetNavItem['labelKey'],
    fallbackLetter: string,
  ): { firstCharacter: string; remainingText: string; fullLabel: string } {
    const fullLabel = t(labelKey).trim();
    const characters = Array.from(fullLabel);

    if (characters.length === 0) {
      return {
        firstCharacter: fallbackLetter,
        remainingText: '',
        fullLabel: fallbackLetter,
      };
    }

    return {
      firstCharacter: characters[0],
      remainingText: characters.slice(1).join(''),
      fullLabel,
    };
  }

  /**
   * Update button text based on current language.
   */
  private updateButtonTitles(): void {
    this.element.querySelectorAll('.transnet-nav__btn').forEach((button) => {
      const btn = button as HTMLButtonElement;
      const letterElement = btn.querySelector('.transnet-nav__btn-letter') as HTMLElement | null;
      const fullTextElement = btn.querySelector('.transnet-nav__btn-full') as HTMLElement | null;
      const labelKey = btn.dataset.labelKey as TransnetNavItem['labelKey'] | undefined;
      const route = btn.dataset.route;
      const item = this.navItems.find((navItem) => navItem.route === route);

      if (!letterElement || !fullTextElement || !labelKey || !item) {
        return;
      }

      const { firstCharacter, remainingText, fullLabel } = this.getLabelParts(labelKey, item.fallbackLetter);

      letterElement.textContent = firstCharacter;
      fullTextElement.textContent = remainingText;
      btn.setAttribute('aria-label', fullLabel);
      btn.title = fullLabel;

      const labelWidth = Math.max(fullTextElement.scrollWidth, 1);
      fullTextElement.style.setProperty('--transnet-nav-label-width', `${labelWidth}px`);
    });
  }

  /**
   * Bind click navigation.
   */
  private bindEvents(): void {
    this.element.querySelectorAll('.transnet-nav__btn').forEach((btn) => {
      btn.addEventListener('click', (event) => {
        event.preventDefault();
        const route = (btn as HTMLElement).getAttribute('data-route');
        if (route && !this.isActive(route)) {
          router.navigate(route);
        }
      });
    });
  }

  /**
   * Mount the nav into a page container.
   */
  mount(parent: HTMLElement): void {
    parent.appendChild(this.element);
    this.updateButtonTitles();
  }

  /**
   * Tear down listeners and remove from DOM.
   */
  destroy(): void {
    this.unsubscribeLanguage?.();
    this.unsubscribeLanguage = null;
    this.element.remove();
  }
}
