/**
 * Minimal pathname-based SPA router.
 *
 * Routes are matched against normalized pathnames so trailing slashes do not
 * create duplicate route registrations.
 */
class Router {
  private routes: Map<string, () => void>;

  constructor() {
    this.routes = new Map();
    this.setupEventListeners();
  }

  /**
   * Register a route handler using the router's normalized path format.
   */
  register(path: string, handler: () => void): void {
    this.routes.set(this.normalizePath(path), handler);
  }

  /**
   * Resolve the current location and execute the matching route handler.
   */
  handleRoute(): void {
    const path = this.normalizePath(window.location.pathname);
    const handler = this.routes.get(path);

    if (handler) {
      handler();
    } else {
      this.routes.get('/home')?.();
    }
  }

  /**
   * Change browser history and immediately resolve the new route.
   */
  navigate(path: string, options: { replace?: boolean } = {}): void {
    if (options.replace) {
      window.history.replaceState({}, '', path);
    } else {
      window.history.pushState({}, '', path);
    }
    this.handleRoute();
  }

  /**
   * Go back in browser history.
   */
  back(): void {
    window.history.back();
  }

  /**
   * Strip a trailing slash from non-root paths so routes stay canonical.
   */
  private normalizePath(path: string): string {
    if (path.length > 1 && path.endsWith('/')) {
      return path.slice(0, -1);
    }

    return path;
  }

  /**
   * Re-run route resolution when the user navigates browser history.
   */
  private setupEventListeners(): void {
    window.addEventListener('popstate', () => {
      this.handleRoute();
    });
  }
}

const router = new Router();
export { router };
