/**
 * Universal application header with home-nav style.
 *
 * This component renders the home-nav header structure across all pages,
 * with active state highlighting only on Home/About/Setting routes.
 */

import { AuthService } from '../services/auth';
import { router } from '../router';
import { t, getCurrentLanguage, setLanguage, onLanguageChange } from '../services/language';

export class Header {
  private element: HTMLElement;
  private unsubscribeAuth: (() => void) | null = null;
  private unsubscribeLanguage: (() => void) | null = null;

  constructor() {
    this.element = document.createElement('header');
    this.element.className = 'home-nav';
    this.render();

    this.unsubscribeAuth = AuthService.onAuthChange(() => {
      this.updateAuthSection();
    });

    this.unsubscribeLanguage = onLanguageChange(() => {
      this.updateLanguage();
    });
  }

  /**
   * Render the full header shell and bind its interactive controls.
   */
  private render(): void {
    this.element.innerHTML = `
      <div class="home-nav__section home-nav__section--left">
        <button type="button" class="home-nav__link ${this.isActive('/home')}" data-route="/home">${t('home')}</button>
        <button type="button" class="home-nav__link ${this.isActive('/about')}" data-route="/about">${t('about')}</button>
        <button type="button" class="home-nav__link ${this.isActive('/setting')}" data-route="/setting">${t('setting')}</button>
      </div>
      <div class="home-nav__section home-nav__section--right">
        <select class="home-lang-select" aria-label="Language">
          <option value="en" ${getCurrentLanguage() === 'en' ? 'selected' : ''}>EN</option>
          <option value="zh" ${getCurrentLanguage() === 'zh' ? 'selected' : ''}>CN</option>
        </select>
        <button type="button" class="home-theme-toggle" aria-label="Dark mode only">
          <span class="home-theme-toggle__track">
            <span class="home-theme-toggle__thumb"></span>
          </span>
          <span class="home-theme-toggle__label">${t('dark')}</span>
        </button>
        <div class="home-auth" aria-live="polite"></div>
      </div>
    `;

    this.bindEvents();
    this.updateAuthSection();
  }

  /**
   * Compare the current location with a target route using normalized paths.
   * Only highlights active state for Home/About/Setting routes.
   */
  private isActive(path: string): string {
    const currentPath = this.normalizePath(window.location.pathname);
    const targetPath = this.normalizePath(path);

    // Only highlight active state for Home/About/Setting routes
    return currentPath === targetPath &&
           (targetPath === '/home' || targetPath === '/about' || targetPath === '/setting')
      ? 'home-nav__link--active'
      : '';
  }

  /**
   * Swap between login/signup buttons and the signed-in user chip.
   */
  private updateAuthSection(): void {
    const authContainer = this.element.querySelector('.home-auth');
    if (!(authContainer instanceof HTMLElement)) {
      return;
    }

    const user = AuthService.getCurrentUser();
    if (user) {
      authContainer.innerHTML = `
        <button type="button" class="home-user-chip" data-route="/transnet/profile">
          <span class="home-user-chip__avatar">${user.username.charAt(0).toUpperCase()}</span>
          <span class="home-user-chip__name">${this.escapeHtml(user.username)}</span>
        </button>
      `;
      return;
    }

    authContainer.innerHTML = `
      <button type="button" class="home-nav__link" data-route="/login">${t('login')}</button>
      <button type="button" class="home-nav__link" data-route="/register">${t('signup')}</button>
    `;
  }

  /**
   * Update language selector when language changes.
   */
  private updateLanguage(): void {
    const langSelect = this.element.querySelector('.home-lang-select') as HTMLSelectElement;
    if (langSelect) {
      langSelect.value = getCurrentLanguage();
    }
  }

  /**
   * Wire SPA navigation and language/theme controls.
   */
  private bindEvents(): void {
    // Handle route button clicks
    this.element.querySelectorAll('[data-route]').forEach((link) => {
      link.addEventListener('click', (event) => {
        event.preventDefault();
        const route = (link as HTMLElement).getAttribute('data-route');
        if (route) {
          router.navigate(route);
        }
      });
    });

    // Handle language dropdown change
    const langSelect = this.element.querySelector('.home-lang-select') as HTMLSelectElement;
    if (langSelect) {
      langSelect.addEventListener('change', (event) => {
        const target = event.target as HTMLSelectElement;
        setLanguage(target.value as 'en' | 'zh');
      });
    }

    // Handle theme toggle (for now, just visual since dark mode is fixed)
    const themeToggle = this.element.querySelector('.home-theme-toggle');
    if (themeToggle) {
      themeToggle.addEventListener('click', () => {
        // Dark mode is currently fixed, so this is visual only
        console.log('Theme toggle clicked (dark mode currently fixed)');
      });
    }
  }

  /**
   * Escape user-provided text before inserting it into markup.
   */
  private escapeHtml(text: string): string {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
  }

  /**
   * Normalize trailing slashes so active-state checks stay consistent.
   */
  private normalizePath(path: string): string {
    if (path.length > 1 && path.endsWith('/')) {
      return path.slice(0, -1);
    }

    return path;
  }

  /**
   * Mount the header into a page container.
   */
  mount(parent: HTMLElement): void {
    parent.appendChild(this.element);
  }

  /**
   * Tear down listeners and remove the header from the DOM.
   */
  destroy(): void {
    this.unsubscribeAuth?.();
    this.unsubscribeAuth = null;
    this.unsubscribeLanguage?.();
    this.unsubscribeLanguage = null;
    this.element.remove();
  }
}
