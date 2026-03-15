/**
 * Profile page for authenticated Transnet users.
 */

import { router } from '../router';
import { ApiService, Profile as ProfileType } from '../services/tran_api/api';
import { AuthService } from '../services/auth';
import { PageShell } from '../shared/page-shell';
import { Toast } from '../shared/toast';

export class Profile {
  private container: HTMLElement;
  private shell: PageShell | null = null;
  private profile: ProfileType | null = null;
  private isEditing = false;
  private mainElement: HTMLElement | null = null;

  constructor(container: HTMLElement = document.body) {
    this.container = container;
    this.container.innerHTML = '';
  }

  /**
   * Guard access, load the profile, and render the editable account page.
   */
  async render(): Promise<void> {
    this.shell = new PageShell(this.container, {
      requiresAuth: true,
      showFooter: false,
      mainClassName: 'profile-page',
    });

    this.mainElement = this.shell.mount();

    await this.loadProfile();

    this.renderContent();
    this.bindEvents();
  }

  /**
   * Render page content only, preserving shell structure.
   */
  private renderContent(): void {
    if (!this.mainElement) return;

    this.mainElement.innerHTML = `
      <div class="container">
        ${this.renderProfileHeader()}

        <div class="profile-grid">
          <section class="glass-card profile-section">
            <h2 class="profile-section__title">Account Settings</h2>

            <form class="profile-form">
              <div class="input-group">
                <label class="input-label">用户名</label>
                <input type="text" class="input-field profile-username" value="${this.profile?.username || ''}" ${!this.isEditing ? 'disabled' : ''}>
              </div>

              <div class="input-group">
                <label class="input-label">邮箱</label>
                <input type="email" class="input-field profile-email" value="${this.profile?.email || ''}" ${!this.isEditing ? 'disabled' : ''}>
              </div>

              ${
                this.isEditing
                  ? `
                <div class="profile-form__actions">
                  <button type="submit" class="btn btn--primary profile-form__grow">Save Changes</button>
                  <button type="button" class="btn btn--ghost profile-cancel">Cancel</button>
                </div>
              `
                  : `
                <button type="button" class="btn btn--secondary profile-edit profile-form__full">
                  Edit Profile
                </button>
              `
              }
            </form>
          </section>

          <section class="glass-card profile-section">
            <h2 class="profile-section__title">Change Password</h2>

            <form class="password-form">
              <div class="input-group">
                <label class="input-label">Current Password</label>
                <input type="password" class="input-field current-password" placeholder="Enter current password" required>
              </div>

              <div class="input-group">
                <label class="input-label">New Password</label>
                <input type="password" class="input-field new-password" placeholder="At least 8 chars with uppercase, lowercase and number" required minlength="8">
              </div>

              <div class="input-group">
                <label class="input-label">Confirm New Password</label>
                <input type="password" class="input-field confirm-password" placeholder="Re-enter new password" required>
              </div>

              <button type="submit" class="btn btn--primary profile-form__full">
                Update Password
              </button>
            </form>
          </section>
        </div>

        <section class="glass-card profile-section profile-section--danger">
          <h2 class="profile-section__title profile-section__title--danger">Danger Zone</h2>
          <button class="btn btn--danger profile-logout profile-form__grow">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4"></path>
              <polyline points="16 17 21 12 16 7"></polyline>
              <line x1="21" y1="12" x2="9" y2="12"></line>
            </svg>
            Logout
          </button>
        </section>
      </div>
    `;
  }

  /**
   * Fetch the latest profile data used by the page.
   */
  private async loadProfile(): Promise<void> {
    const response = await ApiService.getProfile();
    if (response.success && response.data) {
      this.profile = response.data;
    } else {
      Toast.error('Failed to load user info');
    }
  }

  /**
   * Render the profile summary block shown above the editable sections.
   */
  private renderProfileHeader(): string {
    const user = AuthService.getCurrentUser();
    const stats = this.profile?.stats;

    return `
      <div class="profile-header">
        <div class="profile-header__avatar">
          ${user?.username.charAt(0).toUpperCase() || '?'}
        </div>
        <div class="profile-header__info">
          <h1 class="profile-header__name">${user?.username || 'User'}</h1>
          <p class="profile-header__email">${user?.email || ''}</p>
          <div class="profile-header__stats">
            <div class="profile-header__stat">
              <div class="profile-header__stat-value">${stats?.total_translations || 0}</div>
              <div class="profile-header__stat-label">Translations</div>
            </div>
            <div class="profile-header__stat">
              <div class="profile-header__stat-value">${stats?.total_favorites || 0}</div>
              <div class="profile-header__stat-label">Favorites</div>
            </div>
            <div class="profile-header__stat">
              <div class="profile-header__stat-value">${stats?.languages_used?.length || 0}</div>
              <div class="profile-header__stat-label">Languages</div>
            </div>
          </div>
        </div>
      </div>
    `;
  }

  /**
   * Bind profile edit, password update, and logout actions.
   */
  private bindEvents(): void {
    this.mainElement?.querySelector('.profile-edit')?.addEventListener('click', () => {
      this.isEditing = true;
      this.renderContent();
      this.bindEvents();
    });

    this.mainElement?.querySelector('.profile-cancel')?.addEventListener('click', () => {
      this.isEditing = false;
      this.renderContent();
      this.bindEvents();
    });

    this.mainElement?.querySelector('.profile-form')?.addEventListener('submit', async (event) => {
      event.preventDefault();
      await this.saveProfile();
    });

    this.mainElement?.querySelector('.password-form')?.addEventListener('submit', async (event) => {
      event.preventDefault();
      await this.changePassword();
    });

    this.mainElement?.querySelector('.profile-logout')?.addEventListener('click', async () => {
      if (confirm('Are you sure you want to logout?')) {
        await AuthService.logout();
        router.navigate('/transnet');
      }
    });
  }

  /**
   * Save the editable profile fields and refresh cached auth user info.
   */
  private async saveProfile(): Promise<void> {
    const username = (this.mainElement?.querySelector('.profile-username') as HTMLInputElement)?.value;
    const email = (this.mainElement?.querySelector('.profile-email') as HTMLInputElement)?.value;

    if (!username || !email) {
      Toast.error('请填写完整信息');
      return;
    }

    const response = await ApiService.updateProfile({ username, email });
    if (response.success) {
      Toast.success('Profile updated');
      this.isEditing = false;
      await AuthService.updateUserInfo();
      await this.loadProfile();
      this.renderContent();
      this.bindEvents();
    } else {
      Toast.error(response.error?.message || 'Update failed');
    }
  }

  /**
   * Validate and submit a password change request.
   */
  private async changePassword(): Promise<void> {
    const currentPassword = (this.mainElement?.querySelector('.current-password') as HTMLInputElement)?.value;
    const newPassword = (this.mainElement?.querySelector('.new-password') as HTMLInputElement)?.value;
    const confirmPassword = (this.mainElement?.querySelector('.confirm-password') as HTMLInputElement)?.value;

    if (!currentPassword || !newPassword || !confirmPassword) {
      Toast.error('请填写所有密码字段');
      return;
    }

    if (newPassword !== confirmPassword) {
      Toast.error('两次输入的新密码不一致');
      return;
    }

    if (newPassword.length < 8) {
      Toast.error('新密码至少需要8位');
      return;
    }

    const response = await ApiService.changePassword(currentPassword, newPassword);
    if (response.success) {
      Toast.success('Password updated, please login again');
      const form = this.mainElement?.querySelector('.password-form') as HTMLFormElement;
      form?.reset();
    } else {
      Toast.error(response.error?.message || 'Password update failed');
    }
  }

  /**
   * Remove the page and shared widgets from the DOM.
   */
  destroy(): void {
    this.shell?.destroy();
    this.shell = null;
    this.mainElement = null;
  }
}
