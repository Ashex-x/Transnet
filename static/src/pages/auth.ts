/**
 * Auth Page (Login & Register)
 */

import { ParticleBackground } from '../components/ParticleBackground';
import { Header } from '../components/Header';
import { AuthService } from '../services/auth';
import { Toast } from '../components/Toast';
import { router } from '../router';

export class Auth {
  private container: HTMLElement;
  private particleBg: ParticleBackground | null = null;
  private header: Header | null = null;
  private isLogin = true;

  constructor(container: HTMLElement = document.body) {
    this.container = container;
    this.container.innerHTML = '';
  }

  render(): void {
    // Add particle background
    this.particleBg = new ParticleBackground(this.container);

    // Add header
    this.header = new Header();
    this.header.mount(this.container);

    // Create main content
    const main = document.createElement('main');
    main.className = 'auth-page';
    main.innerHTML = `
      <div class="glass-card auth-card animate-fade-in-up">
        <div class="auth-card__header">
          <h1 class="auth-card__title">Welcome Back</h1>
          <p class="auth-card__subtitle">Login or register to save your translation history</p>
        </div>
        
        <div class="auth-card__tabs">
          <button class="auth-card__tab ${this.isLogin ? 'active' : ''}" data-tab="login">Login</button>
          <button class="auth-card__tab ${!this.isLogin ? 'active' : ''}" data-tab="register">Register</button>
        </div>
        
        <form class="auth-form" id="auth-form">
          ${this.renderFormFields()}
          <button type="submit" class="btn btn--primary btn--lg" style="width: 100%; margin-top: 24px;">
            ${this.isLogin ? 'Login' : 'Register'}
          </button>
        </form>
      </div>
    `;

    this.container.appendChild(main);
    this.bindEvents();
  }

  private renderFormFields(): string {
    if (this.isLogin) {
      return `
        <div class="input-group">
          <label class="input-label">Email</label>
          <input type="email" class="input-field" name="email" placeholder="your@email.com" required>
        </div>
        <div class="input-group">
          <label class="input-label">Password</label>
          <input type="password" class="input-field" name="password" placeholder="Enter password" required minlength="8">
        </div>
      `;
    } else {
      return `
        <div class="input-group">
          <label class="input-label">Username</label>
          <input type="text" class="input-field" name="username" placeholder="3-50 chars, alphanumeric and underscore" required minlength="3" maxlength="50" pattern="[a-zA-Z0-9_]+">
        </div>
        <div class="input-group">
          <label class="input-label">Email</label>
          <input type="email" class="input-field" name="email" placeholder="your@email.com" required>
        </div>
        <div class="input-group">
          <label class="input-label">Password</label>
          <input type="password" class="input-field" name="password" placeholder="At least 8 chars with uppercase, lowercase and number" required minlength="8">
          <div class="password-strength"></div>
        </div>
      `;
    }
  }

  private bindEvents(): void {
    // Tab switching
    const tabs = this.container.querySelectorAll('.auth-card__tab');
    tabs.forEach(tab => {
      tab.addEventListener('click', () => {
        const tabName = tab.getAttribute('data-tab');
        this.isLogin = tabName === 'login';
        this.render();
      });
    });

    // Form submission
    const form = this.container.querySelector('#auth-form');
    form?.addEventListener('submit', async (e) => {
      e.preventDefault();
      await this.handleSubmit(e.target as HTMLFormElement);
    });

    // Password strength indicator (for register)
    if (!this.isLogin) {
      const passwordInput = this.container.querySelector('input[name="password"]');
      passwordInput?.addEventListener('input', (e) => {
        this.checkPasswordStrength((e.target as HTMLInputElement).value);
      });
    }
  }

  private checkPasswordStrength(password: string): void {
    const strengthEl = this.container.querySelector('.password-strength');
    if (!strengthEl) return;

    let strength = 0;
    if (password.length >= 8) strength++;
    if (password.match(/[a-z]/)) strength++;
    if (password.match(/[A-Z]/)) strength++;
    if (password.match(/[0-9]/)) strength++;

    const colors = ['var(--error)', 'var(--warning)', 'var(--warning)', 'var(--success)', 'var(--success)'];
    const labels = ['Too short', 'Weak', 'Fair', 'Strong', 'Very Strong'];

    strengthEl.innerHTML = `
      <div style="margin-top: 8px; height: 4px; background: rgba(255,255,255,0.1); border-radius: 2px; overflow: hidden;">
        <div style="width: ${(strength / 4) * 100}%; height: 100%; background: ${colors[strength]}; transition: all 0.3s;"></div>
      </div>
      <span style="font-size: 0.75rem; color: ${colors[strength]}; margin-top: 4px; display: block;">${labels[strength]}</span>
    `;
  }

  private async handleSubmit(form: HTMLFormElement): Promise<void> {
    const formData = new FormData(form);
    const submitBtn = form.querySelector('button[type="submit"]') as HTMLButtonElement;
    
    submitBtn.disabled = true;
    submitBtn.innerHTML = '<span class="loading-spinner"></span> 处理中...';

    if (this.isLogin) {
      const email = formData.get('email') as string;
      const password = formData.get('password') as string;

      const result = await AuthService.login(email, password);
      
      if (result.success) {
        Toast.success('登录成功！');
        router.navigate('/');
      } else {
        Toast.error(result.error || '登录失败');
        submitBtn.disabled = false;
        submitBtn.innerHTML = '登录';
      }
    } else {
      const username = formData.get('username') as string;
      const email = formData.get('email') as string;
      const password = formData.get('password') as string;

      const result = await AuthService.register(username, email, password);
      
      if (result.success) {
        Toast.success('注册成功！请登录');
        this.isLogin = true;
        this.render();
      } else {
        Toast.error(result.error || '注册失败');
        submitBtn.disabled = false;
        submitBtn.innerHTML = '注册';
      }
    }
  }

  destroy(): void {
    this.particleBg?.destroy();
    this.header?.destroy();
    this.container.innerHTML = '';
  }
}
