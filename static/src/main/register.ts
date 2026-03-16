/**
 * Register page for new user registration.
 */

import { PageShell } from "../shared/page-shell";
import { Toast } from "../shared/toast";
import { router } from "../router";
import { AuthService } from "../shared/auth";
import { t } from "../shared/language";

export class Register {
  private container: HTMLElement;
  private shell: PageShell | null = null;
  private mainElement: HTMLElement | null = null;

  constructor(container: HTMLElement = document.body) {
    this.container = container;
    this.container.innerHTML = "";
  }

  render(): void {
    this.shell = new PageShell(this.container, {
      requiresAuth: false,
      showFooter: false,
      mainClassName: "auth-page",
    });

    this.mainElement = this.shell.mount();
    this.renderContent();
    this.bindEvents();
  }

  private renderContent(): void {
    if (!this.mainElement) return;

    this.mainElement.innerHTML = `
      <div class="glass-card auth-card animate-fade-in-up">
        <div class="auth-card__header">
          <h1 class="auth-card__title">${t('createAccount')}</h1>
          <p class="auth-card__subtitle">${t('registerToSave')}</p>
        </div>
        <form class="auth-form" id="register-form">
          <div class="input-group">
            <label class="input-label">${t('username')}</label>
            <input type="text" class="input-field" name="username" placeholder="${t('usernamePlaceholder')}" required minlength="3" maxlength="50" pattern="[a-zA-Z0-9_]+" />
          </div>
          <div class="input-group">
            <label class="input-label">${t('email')}</label>
            <input type="email" class="input-field" name="email" placeholder="${t('emailPlaceholder')}" required />
          </div>
          <div class="input-group">
            <label class="input-label">${t('password')}</label>
            <input type="password" class="input-field" name="password" placeholder="${t('passwordPlaceholderRegister')}" required minlength="8" />
            <div class="password-strength"></div>
          </div>
          <button type="submit" class="btn btn--primary btn--lg auth-form__submit">${t('signup')}</button>
        </form>
        <div class="auth-card__switch">
          <span class="auth-card__switch-text">${t('alreadyHaveAccount')}</span>
          <button type="button" class="auth-card__switch-btn" data-route="/login">${t('login')}</button>
        </div>
      </div>
    `;
  }

  private bindEvents(): void {
    const form = this.mainElement?.querySelector("#register-form");
    form?.addEventListener("submit", async (event) => {
      event.preventDefault();
      await this.handleSubmit(event.target as HTMLFormElement);
    });

    const loginBtn = this.mainElement?.querySelector('[data-route="/login"]');
    loginBtn?.addEventListener("click", (event) => {
      event.preventDefault();
      router.navigate("/login");
    });

    const passwordInput = this.mainElement?.querySelector('input[name="password"]');
    passwordInput?.addEventListener("input", (event) => {
      this.checkPasswordStrength((event.target as HTMLInputElement).value);
    });

    this.container.addEventListener("click", (event) => {
      if (event.target === this.container || event.target === this.mainElement) {
        router.back();
      }
    });
  }

  private checkPasswordStrength(password: string): void {
    const strengthEl = this.mainElement?.querySelector(".password-strength");
    if (!strengthEl) {
      return;
    }

    let strength = 0;
    if (password.length >= 8) strength++;
    if (password.match(/[a-z]/)) strength++;
    if (password.match(/[A-Z]/)) strength++;
    if (password.match(/[0-9]/)) strength++;

    const colors = ["var(--error)", "var(--warning)", "var(--warning)", "var(--success)", "var(--success)"];
    const labels = [t('passwordTooShort'), t('passwordWeak'), t('passwordFair'), t('passwordStrong'), t('passwordVeryStrong')];

    strengthEl.innerHTML = `<div class="password-strength__bar"><div class="password-strength__fill" style="width: ${(strength / 4) * 100}%; background: ${colors[strength]};"></div></div><span class="password-strength__label" style="color: ${colors[strength]};">${labels[strength]}</span>`;
  }

  private async handleSubmit(form: HTMLFormElement): Promise<void> {
    const formData = new FormData(form);
    const username = formData.get("username") as string;
    const email = formData.get("email") as string;
    const password = formData.get("password") as string;
    const submitBtn = form.querySelector('button[type="submit"]') as HTMLButtonElement;

    submitBtn.disabled = true;
    submitBtn.innerHTML = `<span class="loading-spinner"></span> ${t('processing')}`;

    const result = await AuthService.register(username, email, password);

    if (result.success) {
      Toast.success(t('registrationSuccessful'));
      router.navigate("/login");
    } else {
      Toast.error(result.error || t('registrationFailed'));
      submitBtn.disabled = false;
      submitBtn.textContent = t('signup');
    }
  }

  destroy(): void {
    this.shell?.destroy();
    this.shell = null;
    this.mainElement = null;
  }
}
