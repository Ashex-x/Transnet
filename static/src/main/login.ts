/**
 * Login page for user authentication.
 */

import { PageShell } from '../shared/page-shell';
import { Toast } from '../shared/toast';
import { router } from '../router';
import { AuthService } from '../shared/auth';
import { t } from '../shared/language';

export class Login {
  private container: HTMLElement;
  private shell: PageShell | null = null;
  private mainElement: HTMLElement | null = null;

  constructor(container: HTMLElement = document.body) {
    this.container = container;
    this.container.innerHTML = '';
  }

  /**
   * Render the login page and bind events.
   */
  render(): void {
    this.shell = new PageShell(this.container, {
      requiresAuth: false,
      showFooter: false,
      mainClassName: 'auth-page',
    });

    this.mainElement = this.shell.mount();
    this.renderContent();
    this.bindEvents();
  }

  /**
   * Render page content.
   */
  private renderContent(): void {
    if (!this.mainElement) return;

    this.mainElement.innerHTML = `
      <div class="glass-card auth-card animate-fade-in-up">
        <div class="auth-card__header">
          <h1 class="auth-card__title">${t('welcomeBack')}</h1>
          <p class="auth-card__subtitle">${t('loginToSave')}</p>
        </div>

        <form class="auth-form" id="login-form">
          <div class="input-group">
            <label class="input-label">${t('email')}</label>
            <input type="email" class="input-field" name="email" placeholder="${t('emailPlaceholder')}" required>
          </div>
          <div class="input-group">
            <label class="input-label">${t('password')}</label>
            <input type="password" class="input-field" name="password" placeholder="${t('passwordPlaceholder')}" required minlength="8">
          </div>
          <button type="submit" class="btn btn--primary btn--lg auth-form__submit">
            ${t('login')}
          </button>
        </form>

        <div class="auth-card__switch">
          <span class="auth-card__switch-text">${t('dontHaveAccount')}</span>
          <button type="button" class="auth-card__switch-btn" data-route="/register">${t('signup')}</button>
        </div>
      </div>
    `;
  }

  /**
   * Bind form submit and navigation events.
   */
  private bindEvents(): void {
    // Handle form submit
    const form = this.mainElement?.querySelector('#login-form');
    form?.addEventListener('submit', async (event) => {
      event.preventDefault();
      await this.handleSubmit(event.target as HTMLFormElement);
    });

    // Handle register button click
    const registerBtn = this.mainElement?.querySelector('[data-route="/register"]');
    registerBtn?.addEventListener('click', (event) => {
      event.preventDefault();
      router.navigate('/register');
    });

    // Handle background click to go back
    this.container.addEventListener('click', (event) => {
      // Only navigate back if clicking outside the auth card
      if (event.target === this.container || event.target === this.mainElement) {
        router.back();
      }
    });
  }

  /**
   * Submit login form and navigate on success.
   */
  private async handleSubmit(form: HTMLFormElement): Promise<void> {
    const formData = new FormData(form);
    const email = formData.get('email') as string;
    const password = formData.get('password') as string;
    const submitBtn = form.querySelector('button[type="submit"]') as HTMLButtonElement;

    submitBtn.disabled = true;
    submitBtn.innerHTML = `<span class="loading-spinner"></span> ${t('processing')}`;

    const result = await AuthService.login(email, password);

    if (result.success) {
      Toast.success(t('loginSuccessful'));
      router.navigate('/transnet');
    } else {
      Toast.error(result.error || t('loginFailed'));
      submitBtn.disabled = false;
      submitBtn.textContent = t('login');
    }
  }

  /**
   * Remove page UI and shared widgets from the DOM.
   */
  destroy(): void {
    this.shell?.destroy();
    this.shell = null;
    this.mainElement = null;
  }
}
