/**
 * Shared page shell that handles common layout concerns for standard pages.
 *
 * This shell provides particle background, universal header, main content area, and
 * optional footer in a consistent way. Pages use config options rather
 * than inheritance to customize shell behavior.
 */

import { ParticleBackground } from './particle-background';
import { Header } from './header';
import { TransnetNav } from './transnet-nav';
import { AuthService } from './auth';
import { Toast } from './toast';
import { router } from '../router';

export interface PageShellOptions {
  /**
   * Whether this page requires authentication. If true, users will be
   * redirected to login page if not authenticated.
   */
  requiresAuth?: boolean;

  /**
   * Whether to show a footer on this page.
   */
  showFooter?: boolean;

  /**
   * CSS class name to apply to the main content container.
   */
  mainClassName?: string;

  /**
   * Whether to show the Transnet secondary navigation bar.
   */
  showTransnetNav?: boolean;
}

export class PageShell {
  private container: HTMLElement;
  private options: Required<PageShellOptions>;
  private particleBackground: ParticleBackground | null = null;
  private header: Header | null = null;
  private transnetNav: TransnetNav | null = null;
  private mainElement: HTMLElement | null = null;
  private footerElement: HTMLElement | null = null;

  private readonly defaultOptions: Required<PageShellOptions> = {
    requiresAuth: false,
    showFooter: false,
    mainClassName: '',
    showTransnetNav: false,
  };

  constructor(container: HTMLElement, options: PageShellOptions = {}) {
    this.container = container;
    this.options = { ...this.defaultOptions, ...options };
  }

  /**
   * Mount to complete page shell and return to main content element.
   *
   * This method:
   * 1. Checks auth requirements and redirects if needed
   * 2. Mounts particle background
   * 3. Mounts universal header
   * 4. Mounts Transnet secondary nav if enabled
   * 5. Creates and returns main content element
   * 6. Mounts footer if enabled
   *
   * @returns The main content HTMLElement where page-specific content should be appended
   */
  mount(): HTMLElement {
    // Clear container for fresh mount
    this.container.innerHTML = '';

    // Handle auth guard before building shell
    if (this.options.requiresAuth && !AuthService.isAuthenticated()) {
      Toast.info('Please login first');
      router.navigate('/login');
      const main = document.createElement('main');
      this.container.appendChild(main);
      return main;
    }

    // Mount particle background
    this.particleBackground = new ParticleBackground(this.container);

    // Mount universal header
    this.header = new Header();
    this.header.mount(this.container);

    // Mount Transnet secondary nav if enabled
    if (this.options.showTransnetNav) {
      this.transnetNav = new TransnetNav();
      this.transnetNav.mount(this.container);
    }

    // Create main content element
    this.mainElement = document.createElement('main');
    if (this.options.mainClassName) {
      this.mainElement.className = this.options.mainClassName;
    }
    this.container.appendChild(this.mainElement);

    // Mount footer if enabled
    if (this.options.showFooter) {
      this.mountFooter();
    }

    return this.mainElement;
  }

  /**
   * Get to main content element for content updates without full remount.
   *
   * Use this for internal page state changes where you want to update
   * content without rebuilding to entire shell.
   */
  getMainElement(): HTMLElement | null {
    return this.mainElement;
  }

  /**
   * Create and mount a simple footer element.
   */
  private mountFooter(): void {
    this.footerElement = document.createElement('footer');
    this.footerElement.className = 'page-footer';
    this.footerElement.innerHTML = `
      <div class="container">
        <p>Transnet — Bridging AI Worlds</p>
      </div>
    `;
    this.container.appendChild(this.footerElement);
  }

  /**
   * Update to main content with new HTML.
   *
   * This is useful for partial page updates where you want to preserve
   * shell structure but replace content.
   */
  updateContent(html: string): void {
    if (this.mainElement) {
      this.mainElement.innerHTML = html;
    }
  }

  /**
   * Clean up all shell components and remove from DOM.
   *
   * Call this when navigating away from a page to prevent memory leaks
   * and duplicate event listeners.
   */
  destroy(): void {
    this.particleBackground?.destroy();
    this.particleBackground = null;

    this.header?.destroy();
    this.header = null;

    this.transnetNav?.destroy();
    this.transnetNav = null;

    this.footerElement?.remove();
    this.footerElement = null;

    this.mainElement?.remove();
    this.mainElement = null;

    // Clear container completely
    this.container.innerHTML = '';
  }
}
