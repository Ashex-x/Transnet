/**
 * app.ts - Main entry point for the Transnet frontend application
 */

import { router } from './router';
import { HomePage } from './pages/HomePage';
import { AuthPage } from './pages/AuthPage';
import { HistoryPage } from './pages/HistoryPage';
import { FavoritesPage } from './pages/FavoritesPage';
import { ProfilePage } from './pages/ProfilePage';

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

  init(): void {
    // Register routes
    router.register('/', () => this.loadHomePage());
    router.register('/login', () => this.loadAuthPage());
    router.register('/register', () => this.loadAuthPage());
    router.register('/history', () => this.loadHistoryPage());
    router.register('/favorites', () => this.loadFavoritesPage());
    router.register('/profile', () => this.loadProfilePage());

    // Handle initial route
    router.handleRoute();
  }

  private destroyCurrentPage(): void {
    if (this.currentPage) {
      this.currentPage.destroy();
      this.currentPage = null;
    }
  }

  private async loadHomePage(): Promise<void> {
    this.destroyCurrentPage();
    const page = new HomePage(this.container);
    this.currentPage = page;
    await page.render();
  }

  private loadAuthPage(): void {
    this.destroyCurrentPage();
    const page = new AuthPage(this.container);
    this.currentPage = page;
    page.render();
  }

  private async loadHistoryPage(): Promise<void> {
    this.destroyCurrentPage();
    const page = new HistoryPage(this.container);
    this.currentPage = page;
    await page.render();
  }

  private async loadFavoritesPage(): Promise<void> {
    this.destroyCurrentPage();
    const page = new FavoritesPage(this.container);
    this.currentPage = page;
    await page.render();
  }

  private async loadProfilePage(): Promise<void> {
    this.destroyCurrentPage();
    const page = new ProfilePage(this.container);
    this.currentPage = page;
    await page.render();
  }
}

// Initialize app when DOM is ready
const app = new App();
document.addEventListener('DOMContentLoaded', () => {
  app.init();
});

export { App, app };
