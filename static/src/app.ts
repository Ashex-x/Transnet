/**
 * Frontend application bootstrapper.
 *
 * This file wires the SPA router to the active route-page classes and manages
 * page lifecycle transitions.
 */

import { router } from './router';
import { Home } from './main/home';
import { Login } from './main/login';
import { Register } from './main/register';
import { About } from './main/about';
import { Setting } from './main/setting';
import { History } from './transnet/history';
import { Favorites } from './transnet/favorites';
import { Profile } from './transnet/profile';
// import { Metaland } from './metaland/metaland';
import { House } from './metaland/house';
import { Transnet } from './transnet/transnet';

// Note: Styles are loaded via link tag in index.html

interface PageComponent {
  render(): void | Promise<void>;
  destroy(): void;
}

class App {
  private currentPage: PageComponent | null = null;
  private container: HTMLElement;

  constructor() {
    this.container = document.body;
  }

  /**
   * Register all active SPA routes and resolve the initial location.
   */
  init(): void {
    router.register('/', () => this.redirectToHome());
    router.register('/home', () => this.loadHomePage());
    router.register('/login', () => this.loadLoginPage());
    router.register('/register', () => this.loadRegisterPage());
    router.register('/about', () => this.loadAboutPage());
    router.register('/setting', () => this.loadSettingPage());
    router.register('/transnet', () => this.loadTransnetPage());
    router.register('/transnet/history', () => this.loadHistoryPage());
    router.register('/transnet/favorites', () => this.loadFavoritesPage());
    router.register('/transnet/profile', () => this.loadProfilePage());
    router.register('/metaland', () => this.loadMetalandPage());
    router.register('/metaland/house', () => this.loadHousePage());

    router.handleRoute();
  }

  /**
   * Destroy the current page before mounting the next route target.
   */
  private destroyCurrentPage(): void {
    if (this.currentPage) {
      this.currentPage.destroy();
      this.currentPage = null;
    }
  }

  private async loadHomePage(): Promise<void> {
    this.destroyCurrentPage();
    const page = new Home(this.container);
    this.currentPage = page;
    await page.render();
  }

  /**
   * Normalize the root URL by redirecting it to the landing page route.
   */
  private redirectToHome(): void {
    router.navigate('/home', { replace: true });
  }

  /**
   * Mount the login page.
   */
  private loadLoginPage(): void {
    this.destroyCurrentPage();
    const page = new Login(this.container);
    this.currentPage = page;
    page.render();
  }

  /**
   * Mount the register page.
   */
  private loadRegisterPage(): void {
    this.destroyCurrentPage();
    const page = new Register(this.container);
    this.currentPage = page;
    page.render();
  }

  /**
   * Mount the About page.
   */
  private async loadAboutPage(): Promise<void> {
    this.destroyCurrentPage();
    const page = new About(this.container);
    this.currentPage = page;
    await page.render();
  }

  /**
   * Mount the Settings page.
   */
  private async loadSettingPage(): Promise<void> {
    this.destroyCurrentPage();
    const page = new Setting(this.container);
    this.currentPage = page;
    await page.render();
  }

  /**
   * Mount the authenticated translation-history route.
   */
  private async loadHistoryPage(): Promise<void> {
    this.destroyCurrentPage();
    const page = new History(this.container);
    this.currentPage = page;
    await page.render();
  }

  /**
   * Mount the authenticated favorites route.
   */
  private async loadFavoritesPage(): Promise<void> {
    this.destroyCurrentPage();
    const page = new Favorites(this.container);
    this.currentPage = page;
    await page.render();
  }

  /**
   * Mount the authenticated profile route.
   */
  private async loadProfilePage(): Promise<void> {
    this.destroyCurrentPage();
    const page = new Profile(this.container);
    this.currentPage = page;
    await page.render();
  }

  /**
   * Mount the standalone Transnet translation page.
   */
  private async loadTransnetPage(): Promise<void> {
    this.destroyCurrentPage();
    const page = new Transnet(this.container);
    this.currentPage = page;
    await page.render();
  }

  /**
   * Mount the Metaland world entry page.
   */
  private loadMetalandPage(): void {
  }

  /**
   * Mount the Metaland house sub-route.
   */
  private async loadHousePage(): Promise<void> {
    this.destroyCurrentPage();
    const page = new House(this.container);
    this.currentPage = page;
    await page.render();
  }
}

const app = new App();
document.addEventListener('DOMContentLoaded', () => {
  app.init();
});

export { App, app };
