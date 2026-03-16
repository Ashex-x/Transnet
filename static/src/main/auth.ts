/**
 * Root authentication page handling both login and registration flows.
 */

import { PageShell } from '../shared/page-shell';
import { Toast } from '../shared/toast';
import { router } from '../router';
import { AuthService } from '../shared/auth';
import { t } from '../shared/language';

export class Auth {
  private container: HTMLElement;
  private shell: PageShell | null = null;
  private isLogin = true;
  private mainElement: HTMLElement | null = null;

  constructor(container: HTMLElement = document.body) {
    this.container = container;
    this.container.innerHTML = '';
  }

  /**
   * Render the current auth mode and bind the form interactions.
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
   * Render page content only, preserving shell structure.
   */
  private renderContent(): void {
    if (!this.mainElement) return;

    this.mainElement.innerHTML = `
      <div class="glass-card auth-card animate-fade-in-up">
        <div class="auth-card__header">
          <h1 class="auth-card__title">${t('welcomeBack')}</h1>
          <p class="auth-card__subtitle">${t('loginOrRegister')}</p>
        </div>

        <div class="auth-card__tabs">
          <button class="auth-card__tab ${this.isLogin ? 'active' : ''}" data-tab="login">${t('login')}</button>
          <button class="auth-card__tab ${!this.isLogin ? 'active' : ''}" data-tab="register">${t('signup')}</button>
        </div>

        <form class="auth-form" id="auth-form">
          ${this.renderFormFields()}
          <button type="submit" class="btn btn--primary btn--lg auth-form__submit">
            ${this.isLogin ? t('login') : t('signup')}
          </button>
        </form>
      </div>
    `;
  }

  /**
   * Render the fields needed for the active auth mode.
   */
  private renderFormFields(): string {
    if (this.isLogin) {
      return `
        <div class="input-group">
          <label class="input-label">${t('email')}</label>
          <input type="email" class="input-field" name="email" placeholder="${t('emailPlaceholder')}" required>
        </div>
        <div class="input-group">
          <label class="input-label">${t('password')}</label>
          <input type="password" class="input-field" name="password" placeholder="${t('passwordPlaceholder')}" required minlength="8">
        </div>
      `;
    }

    return `
      <div class="input-group">
        <label class="input-label">${t('username')}</label>
        <input type="text" class="input-field" name="username" placeholder="${t('usernamePlaceholder')}" required minlength="3" maxlength="50" pattern="[a-zA-Z0-9_]+">
      </div>
      <div class="input-group">
        <label class="input-label">${t('email')}</label>
        <input type="email" class="input-field" name="email" placeholder="${t('emailPlaceholder')}" required>
      </div>
      <div class="input-group">
        <label class="input-label">${t('password')}</label>
        <input type="password" class="input-field" name="password" placeholder="${t('passwordPlaceholderRegister')}" required minlength="8">
        <div class="password-strength"></div>
      </div>
    `;
  }

  /**
   * Bind tab switching, submit handling, and password strength updates.
   */
  private bindEvents(): void {
    const tabs = this.mainElement?.querySelectorAll('.auth-card__tab');
    tabs?.forEach((tab) => {
      tab.addEventListener('click', () => {
        this.isLogin = tab.getAttribute('data-tab') === 'login';
        this.renderContent();
        this.bindEvents();
      });
    });

    const form = this.mainElement?.querySelector('#auth-form');
    form?.addEventListener('submit', async (event) => {
      event.preventDefault();
      await this.handleSubmit(event.target as HTMLFormElement);
    });

    if (!this.isLogin) {
      const passwordInput = this.mainElement?.querySelector('input[name="password"]');
      passwordInput?.addEventListener('input', (event) => {
        this.checkPasswordStrength((event.target as HTMLInputElement).value);
      });
    }
  }

  /**
   * Render a simple password strength indicator for the register form.
   */
  private checkPasswordStrength(password: string): void {
    const strengthEl = this.mainElement?.querySelector('.password-strength');
    if (!strengthEl) {
      return;
    }

    let strength = 0;
    if (password.length >= 8) strength++;
    if (password.match(/[a-z]/)) strength++;
    if (password.match(/[A-Z]/)) strength++;
    if (password.match(/[0-9]/)) strength++;

    const colors = ['var(--error)', 'var(--warning)', 'var(--warning)', 'var(--success)', 'var(--success)'];
    const labels = [t('passwordTooShort'), t('passwordWeak'), t('passwordFair'), t('passwordStrong'), t('passwordVeryStrong')];

    strengthEl.innerHTML = `
      <div class="password-strength__bar">
        <div class="password-strength__fill" style="width: ${(strength / 4) * 100}%; background: ${colors[strength]};"></div>
      </div>
      <span class="password-strength__label" style="color: ${colors[strength]};">${labels[strength]}</span>
    `;
  }

  /**
   * Submit the active auth form and route on success.
   */
  private async handleSubmit(form: HTMLFormElement): Promise<void> {
    const formData = new FormData(form);
    const submitBtn = form.querySelector('button[type="submit"]') as HTMLButtonElement;

    submitBtn.disabled = true;
    submitBtn.innerHTML = `<span class="loading-spinner"></span> ${t('processing')}`;

    if (this.isLogin) {
      const email = formData.get('email') as string;
      const password = formData.get('password') as string;
      const result = await AuthService.login(email, password);

      if (result.success) {
        Toast.success(t('loginSuccessful'));
        router.navigate('/transnet');
      } else {
        Toast.error(result.error || t('loginFailed'));
        submitBtn.disabled = false;
        submitBtn.textContent = t('login');
      }

      return;
    }

    const username = formData.get('username') as string;
    const email = formData.get('email') as string;
    const password = formData.get('password') as string;
    const result = await AuthService.register(username, email, password);

    if (result.success) {
      Toast.success(t('registrationSuccessful'));
      this.isLogin = true;
      this.renderContent();
      this.bindEvents();
    } else {
      Toast.error(result.error || t('registrationFailed'));
      submitBtn.disabled = false;
      submitBtn.textContent = t('signup');
    }
  }

  /**
   * Remove the page UI and shared widgets from the DOM.
   */
  destroy(): void {
    this.shell?.destroy();
    this.shell = null;
    this.mainElement = null;
  }
}
