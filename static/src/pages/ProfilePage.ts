/**
 * Profile Page
 * User profile and settings
 */

import { ParticleBackground } from '../components/ParticleBackground';
import { Header } from '../components/Header';
import { ApiService, Profile } from '../services/ApiService';
import { AuthService } from '../services/AuthService';
import { Toast } from '../components/Toast';
import { router } from '../router';

export class ProfilePage {
  private container: HTMLElement;
  private particleBg: ParticleBackground | null = null;
  private header: Header | null = null;
  private profile: Profile | null = null;
  private isEditing = false;

  constructor(container: HTMLElement = document.body) {
    this.container = container;
    this.container.innerHTML = '';
  }

  async render(): Promise<void> {
    // Check authentication
    if (!AuthService.isAuthenticated()) {
      Toast.info('Please login first');
      router.navigate('/login');
      return;
    }

    // Add particle background
    this.particleBg = new ParticleBackground(this.container);

    // Add header
    this.header = new Header();
    this.header.mount(this.container);

    // Load profile data
    await this.loadProfile();

    // Create main content
    const main = document.createElement('main');
    main.className = 'profile-page';
    main.innerHTML = `
      <div class="container">
        ${this.renderProfileHeader()}
        
        <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 24px;">
          <!-- Account Settings -->
          <div class="glass-card" style="padding: 32px;">
            <h2 style="font-size: 1.25rem; font-weight: 600; margin-bottom: 24px;">Account Settings</h2>
            
            <form class="profile-form">
              <div class="input-group">
                <label class="input-label">用户名</label>
                <input type="text" class="input-field profile-username" value="${this.profile?.username || ''}" ${!this.isEditing ? 'disabled' : ''}>
              </div>
              
              <div class="input-group">
                <label class="input-label">邮箱</label>
                <input type="email" class="input-field profile-email" value="${this.profile?.email || ''}" ${!this.isEditing ? 'disabled' : ''}>
              </div>

              ${this.isEditing ? `
                <div style="display: flex; gap: 12px; margin-top: 24px;">
                  <button type="submit" class="btn btn--primary" style="flex: 1;">Save Changes</button>
                  <button type="button" class="btn btn--ghost profile-cancel">Cancel</button>
                </div>
              ` : `
                <button type="button" class="btn btn--secondary profile-edit" style="width: 100%; margin-top: 24px;">
                  Edit Profile
                </button>
              `}
            </form>
          </div>

          <!-- Password Change -->
          <div class="glass-card" style="padding: 32px;">
            <h2 style="font-size: 1.25rem; font-weight: 600; margin-bottom: 24px;">Change Password</h2>
            
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

              <button type="submit" class="btn btn--primary" style="width: 100%; margin-top: 24px;">
                Update Password
              </button>
            </form>
          </div>
        </div>

        <!-- Account Actions -->
        <div class="glass-card" style="padding: 32px; margin-top: 24px;">
          <h2 style="font-size: 1.25rem; font-weight: 600; margin-bottom: 24px; color: var(--error);">Danger Zone</h2>
          <div style="display: flex; gap: 16px;">
            <button class="btn btn--danger profile-logout" style="flex: 1;">
              <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="margin-right: 8px;">
                <path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4"></path>
                <polyline points="16 17 21 12 16 7"></polyline>
                <line x1="21" y1="12" x2="9" y2="12"></line>
              </svg>
              Logout
            </button>
          </div>
        </div>
      </div>
    `;

    this.container.appendChild(main);
    this.bindEvents();
  }

  private async loadProfile(): Promise<void> {
    const response = await ApiService.getProfile();
    if (response.success && response.data) {
      this.profile = response.data;
    } else {
      Toast.error('Failed to load user info');
    }
  }

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

  private bindEvents(): void {
    // Edit profile
    const editBtn = this.container.querySelector('.profile-edit');
    editBtn?.addEventListener('click', () => {
      this.isEditing = true;
      this.render();
    });

    // Cancel edit
    const cancelBtn = this.container.querySelector('.profile-cancel');
    cancelBtn?.addEventListener('click', () => {
      this.isEditing = false;
      this.render();
    });

    // Save profile
    const profileForm = this.container.querySelector('.profile-form');
    profileForm?.addEventListener('submit', async (e) => {
      e.preventDefault();
      await this.saveProfile();
    });

    // Change password
    const passwordForm = this.container.querySelector('.password-form');
    passwordForm?.addEventListener('submit', async (e) => {
      e.preventDefault();
      await this.changePassword();
    });

    // Logout
    const logoutBtn = this.container.querySelector('.profile-logout');
    logoutBtn?.addEventListener('click', async () => {
      if (confirm('Are you sure you want to logout?')) {
        await AuthService.logout();
        router.navigate('/');
      }
    });
  }

  private async saveProfile(): Promise<void> {
    const username = (this.container.querySelector('.profile-username') as HTMLInputElement)?.value;
    const email = (this.container.querySelector('.profile-email') as HTMLInputElement)?.value;

    if (!username || !email) {
      Toast.error('请填写完整信息');
      return;
    }

    const response = await ApiService.updateProfile({ username, email });

    if (response.success) {
      Toast.success('Profile updated');
      this.isEditing = false;
      await AuthService.updateUserInfo();
      this.render();
    } else {
      Toast.error(response.error?.message || 'Update failed');
    }
  }

  private async changePassword(): Promise<void> {
    const currentPassword = (this.container.querySelector('.current-password') as HTMLInputElement)?.value;
    const newPassword = (this.container.querySelector('.new-password') as HTMLInputElement)?.value;
    const confirmPassword = (this.container.querySelector('.confirm-password') as HTMLInputElement)?.value;

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
      // Clear password form
      const form = this.container.querySelector('.password-form') as HTMLFormElement;
      form?.reset();
    } else {
      Toast.error(response.error?.message || 'Password update failed');
    }
  }

  destroy(): void {
    this.particleBg?.destroy();
    this.header?.destroy();
    this.container.innerHTML = '';
  }
}
