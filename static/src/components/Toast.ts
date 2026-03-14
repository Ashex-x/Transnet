/**
 * Toast Notification System
 */

export type ToastType = 'success' | 'error' | 'info';

interface ToastOptions {
  duration?: number;
  type?: ToastType;
}

class ToastClass {
  private container: HTMLElement | null = null;

  private getContainer(): HTMLElement {
    if (!this.container) {
      this.container = document.createElement('div');
      this.container.className = 'toast-container';
      document.body.appendChild(this.container);
    }
    return this.container;
  }

  show(message: string, options: ToastOptions = {}): void {
    const { duration = 3000, type = 'info' } = options;
    const container = this.getContainer();

    const toast = document.createElement('div');
    toast.className = `toast toast--${type}`;

    // Icon based on type
    const icon = this.getIcon(type);

    toast.innerHTML = `
      <span class="toast__icon">${icon}</span>
      <span class="toast__message">${message}</span>
    `;

    container.appendChild(toast);

    // Remove after duration
    setTimeout(() => {
      toast.style.opacity = '0';
      toast.style.transform = 'translateX(100px)';
      toast.style.transition = 'all 0.3s ease';
      
      setTimeout(() => {
        toast.remove();
      }, 300);
    }, duration);
  }

  success(message: string, duration?: number): void {
    this.show(message, { type: 'success', duration });
  }

  error(message: string, duration?: number): void {
    this.show(message, { type: 'error', duration });
  }

  info(message: string, duration?: number): void {
    this.show(message, { type: 'info', duration });
  }

  private getIcon(type: ToastType): string {
    switch (type) {
      case 'success':
        return '✓';
      case 'error':
        return '✕';
      case 'info':
        return 'ℹ';
      default:
        return 'ℹ';
    }
  }
}

export const Toast = new ToastClass();
