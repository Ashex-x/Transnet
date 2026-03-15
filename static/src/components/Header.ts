/**
 * Header Component
 * Navigation and user menu
 */

import { AuthService } from '../services/auth';
import { router } from '../router';

export class Header {
  private element: HTMLElement;
  private unsubscribeAuth: (() => void) | null = null;

  constructor() {
    this.element = document.createElement('header');
    this.element.className = 'header';
    this.render();
    
    // Listen for auth changes
    this.unsubscribeAuth = AuthService.onAuthChange(() => {
      this.updateUserSection();
    });
  }

  private render(): void {
    const isAuth = AuthService.isAuthenticated();
    const user = AuthService.getCurrentUser();

    this.element.innerHTML = `
      <div class="container">
        <div class="header__inner">
          <a href="/" class="header__logo" data-link>
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <polygon points="12 2 22 8.5 22 15.5 12 22 2 15.5 2 8.5 12 2"></polygon>
              <line x1="12" y1="22" x2="12" y2="15.5"></line>
              <polyline points="22 8.5 12 15.5 2 8.5"></polyline>
              <polyline points="2 15.5 12 8.5 22 15.5"></polyline>
              <line x1="12" y1="2" x2="12" y2="8.5"></line>
            </svg>
            Transnet
          </a>
          
          <nav class="header__nav">
            <a href="/" class="header__link ${this.isActive('/')}" data-link>Translate</a>
            <a href="/history" class="header__link ${this.isActive('/history')}" data-link>History</a>
            <a href="/favorites" class="header__link ${this.isActive('/favorites')}" data-link>Favorites</a>
          </nav>
          
          <div class="header__user">
            ${this.renderUserSection(isAuth, user)}
          </div>
        </div>
      </div>
    `;

    this.bindEvents();
  }

  private renderUserSection(isAuth: boolean, user: { username: string } | null): string {
    if (isAuth && user) {
      return `
        <a href="/profile" class="header__link ${this.isActive('/profile')}" data-link>
          <div class="header__avatar">${user.username.charAt(0).toUpperCase()}</div>
        </a>
        <button class="btn btn--ghost btn--icon" id="logout-btn" title="Logout">
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4"></path>
            <polyline points="16 17 21 12 16 7"></polyline>
            <line x1="21" y1="12" x2="9" y2="12"></line>
          </svg>
        </button>
      `;
    } else {
      return `
        <a href="/login" class="btn btn--secondary" data-link>Login</a>
      `;
    }
  }

  private updateUserSection(): void {
    const userContainer = this.element.querySelector('.header__user');
    if (userContainer) {
      const isAuth = AuthService.isAuthenticated();
      const user = AuthService.getCurrentUser();
      userContainer.innerHTML = this.renderUserSection(isAuth, user);
      this.bindEvents();
    }
  }

  private isActive(path: string): string {
    return window.location.pathname === path ? 'active' : '';
  }

  private bindEvents(): void {
    // Handle navigation clicks
    this.element.querySelectorAll('[data-link]').forEach(link => {
      link.addEventListener('click', (e) => {
        e.preventDefault();
        const href = (e.currentTarget as HTMLAnchorElement).getAttribute('href');
        if (href) {
          router.navigate(href);
        }
      });
    });

    // Logout button
    const logoutBtn = this.element.querySelector('#logout-btn');
    if (logoutBtn) {
      logoutBtn.addEventListener('click', async () => {
        await AuthService.logout();
        router.navigate('/');
      });
    }
  }

  mount(parent: HTMLElement): void {
    parent.appendChild(this.element);
  }

  destroy(): void {
    if (this.unsubscribeAuth) {
      this.unsubscribeAuth();
    }
    this.element.remove();
  }
}
