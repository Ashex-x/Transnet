/**
 * Main landing page for application.
 *
 * This page uses PageShell for shared particle background and header,
 * and only owns to sphere-specific content and interaction logic.
 */

import { PageShell } from '../shared/page-shell';
import { HomeSphere } from './home-sphere';
import { t } from '../shared/language';

export class Home {
  private container: HTMLElement;
  private shell: PageShell | null = null;
  private sphere: HomeSphere | null = null;
  private mainElement: HTMLElement | null = null;

  constructor(container: HTMLElement = document.body) {
    this.container = container;
    this.container.innerHTML = '';
  }

  /**
   * Render to landing page and sphere content.
   */
  async render(): Promise<void> {
    this.shell = new PageShell(this.container, {
      requiresAuth: false,
      showFooter: false,
      mainClassName: 'home-page',
    });

    this.mainElement = this.shell.mount();

    // Render Home-specific content (sphere only, no header/nav since PageShell handles it)
    this.mainElement.innerHTML = `
      <main class="home-stage">
        <div class="home-stage__content">
          <p class="home-stage__eyebrow">${t('chooseIsland')}</p>
          <h1 class="home-stage__title">${t('welcome')}</h1>
          <p class="home-stage__subtitle">${t('instructions')}</p>
          <div class="home-sphere-slot"></div>
          <div class="home-stage__footer">
            <span>${t('currentTheme')}</span>
            <span>${t('backgroundPreserved')}</span>
          </div>
        </div>
      </main>
    `;

    // Mount sphere
    const sphereSlot = this.mainElement.querySelector('.home-sphere-slot');
    if (sphereSlot instanceof HTMLElement) {
      this.sphere = new HomeSphere();
      this.sphere.mount(sphereSlot);
    }
  }

  /**
   * Release to mounted widgets and clear to page container.
   */
  destroy(): void {
    this.sphere?.destroy();
    this.sphere = null;
    this.shell?.destroy();
    this.shell = null;
    this.mainElement = null;
  }
}
