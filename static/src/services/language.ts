/**
 * Language service for managing application language state and translations.
 *
 * Provides centralized language management with localStorage persistence
 * and pub/sub notifications for language changes.
 */

type Language = 'en' | 'zh';
type TranslationKey = keyof typeof translations.en;

interface Translations {
  en: Record<string, string>;
  zh: Record<string, string>;
}

const translations: Translations = {
  en: {
    // Home page
    home: 'Home',
    about: 'About',
    setting: 'Setting',
    chooseIsland: 'Choose your island',
    welcome: 'Welcome to Island OS.',
    instructions: 'Right-click and drag to roll the sphere. Hover the named nodes to preview each island, then click to enter.',
    currentTheme: 'Current theme: Dark mode',
    backgroundPreserved: 'Background preserved',
    login: 'Login',
    signup: 'Signup',
    dark: 'Dark',
  },
  zh: {
    home: '首页',
    about: '关于',
    setting: '设置',
    chooseIsland: '选择您的岛屿',
    welcome: '欢迎来到 Island OS。',
    instructions: '右键拖动旋转球体。悬停在命名节点上预览每个岛屿，然后点击进入。',
    currentTheme: '当前主题: 深色模式',
    backgroundPreserved: '背景已保留',
    login: '登录',
    signup: '注册',
    dark: '深色',
  },
};

class LanguageService {
  private currentLanguage: Language;
  private changeListeners: Set<(lang: Language) => void> = new Set();
  private readonly STORAGE_KEY = 'transnet-language';

  constructor() {
    // Load saved language from localStorage or default to 'en'
    const saved = localStorage.getItem(this.STORAGE_KEY);
    this.currentLanguage = (saved === 'en' || saved === 'zh') ? saved : 'en';
    this.updateHtmlLangAttribute();
  }

  /**
   * Get the current language code.
   */
  getCurrentLanguage(): Language {
    return this.currentLanguage;
  }

  /**
   * Set a new language and notify all listeners.
   */
  setLanguage(language: Language): void {
    if (this.currentLanguage === language) {
      return;
    }

    this.currentLanguage = language;
    localStorage.setItem(this.STORAGE_KEY, language);
    this.updateHtmlLangAttribute();
    this.notifyListeners();
  }

  /**
   * Get translation for a key in the current language.
   */
  t(key: TranslationKey): string {
    return translations[this.currentLanguage][key] || translations.en[key] || key;
  }

  /**
   * Subscribe to language changes. Returns an unsubscribe function.
   */
  onLanguageChange(callback: (lang: Language) => void): () => void {
    this.changeListeners.add(callback);

    // Return unsubscribe function
    return () => {
      this.changeListeners.delete(callback);
    };
  }

  /**
   * Notify all registered listeners of language change.
   */
  private notifyListeners(): void {
    this.changeListeners.forEach((callback) => {
      callback(this.currentLanguage);
    });
  }

  /**
   * Update the HTML lang attribute for accessibility.
   */
  private updateHtmlLangAttribute(): void {
    document.documentElement.lang = this.currentLanguage;
  }
}

// Singleton instance
const languageService = new LanguageService();

// Export convenience functions
export const t = (key: TranslationKey): string => languageService.t(key);
export const getCurrentLanguage = (): Language => languageService.getCurrentLanguage();
export const setLanguage = (lang: Language): void => languageService.setLanguage(lang);
export const onLanguageChange = (callback: (lang: Language) => void): (() => void) =>
  languageService.onLanguageChange(callback);

// Export service instance for direct access if needed
export { languageService };
export default languageService;
