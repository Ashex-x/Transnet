/**
 * Shared toast notification helper.
 *
 * The singleton lazily creates its container and exposes small helpers for the
 * supported message types used across the SPA.
 */

export type ToastType = 'success' | 'error' | 'info';

interface ToastOptions {
  duration?: number;
  type?: ToastType;
}

class ToastClass {
  private container: HTMLElement | null = null;

  /**
   * Create the toast container the first time it is needed.
   */
  private getContainer(): HTMLElement {
    if (!this.container) {
      this.container = document.createElement('div');
      this.container.className = 'toast-container';
      document.body.appendChild(this.container);
    }

    return this.container;
  }

  /**
   * Render a toast message and remove it after the configured duration.
   */
  show(message: string, options: ToastOptions = {}): void {
    const { duration = 3000, type = 'info' } = options;
    const container = this.getContainer();

    const toast = document.createElement('div');
    toast.className = `toast toast--${type}`;
    toast.innerHTML = `
      <span class="toast__icon">${this.getIcon(type)}</span>
      <span class="toast__message">${message}</span>
    `;

    container.appendChild(toast);

    setTimeout(() => {
      toast.style.opacity = '0';
      toast.style.transform = 'translateX(100px)';
      toast.style.transition = 'all 0.3s ease';

      setTimeout(() => {
        toast.remove();
      }, 300);
    }, duration);
  }

  /**
   * Show a success toast with the default or supplied duration.
   */
  success(message: string, duration?: number): void {
    this.show(message, { type: 'success', duration });
  }

  /**
   * Show an error toast with the default or supplied duration.
   */
  error(message: string, duration?: number): void {
    this.show(message, { type: 'error', duration });
  }

  /**
   * Show an informational toast with the default or supplied duration.
   */
  info(message: string, duration?: number): void {
    this.show(message, { type: 'info', duration });
  }

  /**
   * Select the glyph used for each toast type.
   */
  private getIcon(type: ToastType): string {
    switch (type) {
      case 'success':
        return '✓';
      case 'error':
        return '✕';
      case 'info':
      default:
        return 'ℹ';
    }
  }
}

export const Toast = new ToastClass();
