/**
 * Translator Component
 * Main translation interface with glass morphism design
 */

import { ApiService, TranslationRequest, Translation } from '../services/api';
import { AuthService } from '../services/auth';
import { Toast } from './Toast';

const LANGUAGES = [
  { code: 'en', name: 'English', flag: '🇺🇸' },
  { code: 'es', name: 'Español', flag: '🇪🇸' },
  { code: 'fr', name: 'Français', flag: '🇫🇷' },
  { code: 'de', name: 'Deutsch', flag: '🇩🇪' },
  { code: 'zh', name: '中文', flag: '🇨🇳' },
  { code: 'ja', name: '日本語', flag: '🇯🇵' },
];

const MAX_TEXT_LENGTH = 5000;

export class Translator {
  private element: HTMLElement;
  private sourceText: HTMLTextAreaElement | null = null;
  private targetOutput: HTMLElement | null = null;
  private sourceLang: HTMLSelectElement | null = null;
  private targetLang: HTMLSelectElement | null = null;
  private charCount: HTMLElement | null = null;
  private translateBtn: HTMLButtonElement | null = null;
  private isLoading = false;
  private lastTranslation: Translation | null = null;

  constructor() {
    this.element = document.createElement('div');
    this.element.className = 'translator';
    this.render();
  }

  private render(): void {
    this.element.innerHTML = `
      <div class="translator__panels">
        <!-- Source Panel -->
        <div class="glass-card translator__panel">
          <div class="translator__header">
            <div class="lang-select">
              <span>🌐</span>
              <select class="translator__source-lang">
                ${LANGUAGES.map(lang => `
                  <option value="${lang.code}">${lang.flag} ${lang.name}</option>
                `).join('')}
              </select>
            </div>
            <button class="btn btn--ghost btn--icon translator__clear-btn" title="Clear">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <line x1="18" y1="6" x2="6" y2="18"></line>
                <line x1="6" y1="6" x2="18" y2="18"></line>
              </svg>
            </button>
          </div>
          <textarea 
            class="translator__textarea" 
            placeholder="Enter text to translate..."
            maxlength="${MAX_TEXT_LENGTH}"
          ></textarea>
          <div class="translator__footer">
            <span class="translator__char-count">0 / ${MAX_TEXT_LENGTH}</span>
            <div class="translator__actions">
              <button class="btn btn--ghost btn--icon translator__paste-btn" title="Paste">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <path d="M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2"></path>
                  <rect x="8" y="2" width="8" height="4" rx="1" ry="1"></rect>
                </svg>
              </button>
            </div>
          </div>
        </div>

        <!-- Swap Button -->
        <div class="translator__center">
          <button class="lang-swap-btn translator__swap-btn" title="Swap languages">
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <polyline points="7 16 17 16 17 8"></polyline>
              <polyline points="17 16 21 12 17 8"></polyline>
              <polyline points="17 8 7 8 7 16"></polyline>
              <polyline points="7 8 3 12 7 16"></polyline>
            </svg>
          </button>
        </div>

        <!-- Target Panel -->
        <div class="glass-card translator__panel">
          <div class="translator__header">
            <div class="lang-select">
              <span>🎯</span>
              <select class="translator__target-lang">
                ${LANGUAGES.map((lang, i) => `
                  <option value="${lang.code}" ${i === 1 ? 'selected' : ''}>${lang.flag} ${lang.name}</option>
                `).join('')}
              </select>
            </div>
            <div class="translator__actions">
              <button class="btn btn--ghost btn--icon translator__copy-btn" title="Copy">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
                  <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
                </svg>
              </button>
              <button class="btn btn--ghost btn--icon translator__favorite-btn" title="Add to favorites">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"></polygon>
                </svg>
              </button>
            </div>
          </div>
          <div class="translator__output">
            <span class="translator__placeholder">Translation will appear here...</span>
          </div>
          <div class="translator__footer">
            <span></span>
            <div class="translator__actions">
              <span class="translator__detected-type"></span>
            </div>
          </div>
        </div>
      </div>

      <div class="translator__submit">
        <button class="btn btn--primary btn--lg translator__translate-btn">
          <span class="btn__text">✨ Translate</span>
          <span class="btn__loading hidden">
            <span class="loading-spinner"></span>
            Translating...
          </span>
        </button>
      </div>
    `;

    this.bindElements();
    this.bindEvents();
  }

  private bindElements(): void {
    this.sourceText = this.element.querySelector('.translator__textarea');
    this.targetOutput = this.element.querySelector('.translator__output');
    this.sourceLang = this.element.querySelector('.translator__source-lang');
    this.targetLang = this.element.querySelector('.translator__target-lang');
    this.charCount = this.element.querySelector('.translator__char-count');
    this.translateBtn = this.element.querySelector('.translator__translate-btn');
  }

  private bindEvents(): void {
    // Character count
    this.sourceText?.addEventListener('input', () => {
      this.updateCharCount();
    });

    // Clear button
    this.element.querySelector('.translator__clear-btn')?.addEventListener('click', () => {
      if (this.sourceText) {
        this.sourceText.value = '';
        this.updateCharCount();
        this.sourceText.focus();
      }
    });

    // Paste button
    this.element.querySelector('.translator__paste-btn')?.addEventListener('click', async () => {
      try {
        const text = await navigator.clipboard.readText();
        if (this.sourceText) {
          this.sourceText.value = text.slice(0, MAX_TEXT_LENGTH);
          this.updateCharCount();
        }
      } catch {
        Toast.error('Unable to access clipboard');
      }
    });

    // Copy button
    this.element.querySelector('.translator__copy-btn')?.addEventListener('click', () => {
      const text = this.targetOutput?.textContent;
      if (text && text !== 'Translation will appear here...') {
        navigator.clipboard.writeText(text);
        Toast.success('Copied to clipboard');
      }
    });

    // Favorite button
    this.element.querySelector('.translator__favorite-btn')?.addEventListener('click', () => {
      this.addToFavorites();
    });

    // Swap languages
    this.element.querySelector('.translator__swap-btn')?.addEventListener('click', () => {
      this.swapLanguages();
    });

    // Translate button
    this.translateBtn?.addEventListener('click', () => {
      this.translate();
    });

    // Enter key to translate (Ctrl/Cmd + Enter)
    this.sourceText?.addEventListener('keydown', (e) => {
      if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') {
        this.translate();
      }
    });
  }

  private updateCharCount(): void {
    const count = this.sourceText?.value.length || 0;
    if (this.charCount) {
      this.charCount.textContent = `${count} / ${MAX_TEXT_LENGTH}`;
      
      // Update warning states
      this.charCount.classList.remove('warning', 'error');
      if (count > MAX_TEXT_LENGTH * 0.9) {
        this.charCount.classList.add('error');
      } else if (count > MAX_TEXT_LENGTH * 0.8) {
        this.charCount.classList.add('warning');
      }
    }
  }

  private swapLanguages(): void {
    if (this.sourceLang && this.targetLang) {
      const temp = this.sourceLang.value;
      this.sourceLang.value = this.targetLang.value;
      this.targetLang.value = temp;
      
      // Also swap text if there's a translation
      if (this.lastTranslation && this.sourceText) {
        this.sourceText.value = this.lastTranslation.translation;
        this.updateCharCount();
        this.showResult(this.lastTranslation.text);
      }
    }
  }

  private async translate(): Promise<void> {
    if (this.isLoading) return;

    const text = this.sourceText?.value.trim();
    if (!text) {
      Toast.error('Please enter text to translate');
      return;
    }

    const sourceLang = this.sourceLang?.value;
    const targetLang = this.targetLang?.value;

    if (!sourceLang || !targetLang) {
      Toast.error('Please select source and target languages');
      return;
    }

    if (sourceLang === targetLang) {
      Toast.error('Source and target languages cannot be the same');
      return;
    }

    this.setLoading(true);

    const request: TranslationRequest = {
      text,
      source_lang: sourceLang,
      target_lang: targetLang,
      mode: 'basic',
      input_type: 'auto',
    };

    const response = await ApiService.translate(request);

    this.setLoading(false);

    if (response.success && response.data) {
      this.lastTranslation = response.data;
      this.showResult(response.data.translation);
      this.showDetectedType(response.data.input_type);
      
      // If user is authenticated, translation is automatically saved to history
      if (response.data.user_id) {
        Toast.success('Translation complete and saved to history');
      } else {
        Toast.success('Translation complete');
      }
    } else {
      Toast.error(response.error?.message || 'Translation failed, please try again later');
    }
  }

  private showResult(text: string): void {
    if (this.targetOutput) {
      this.targetOutput.innerHTML = `<p>${this.escapeHtml(text)}</p>`;
    }
  }

  private showDetectedType(type: string): void {
    const typeEl = this.element.querySelector('.translator__detected-type');
    if (typeEl) {
      const typeMap: Record<string, string> = {
        word: 'Word',
        phrase: 'Phrase',
        sentence: 'Sentence',
        text: 'Text',
      };
      typeEl.textContent = `Detected: ${typeMap[type] || type}`;
    }
  }

  private async addToFavorites(): Promise<void> {
    if (!this.lastTranslation) {
      Toast.info('Please translate first');
      return;
    }

    if (!AuthService.isAuthenticated()) {
      Toast.error('Please login to add favorites');
      return;
    }

    const response = await ApiService.addFavorite({
      translation_id: this.lastTranslation.id,
    });

    if (response.success) {
      Toast.success('Added to favorites');
    } else {
      if (response.error?.code === 'CONFLICT') {
        Toast.info('Already in favorites');
      } else {
        Toast.error(response.error?.message || 'Failed to add favorite');
      }
    }
  }

  private setLoading(loading: boolean): void {
    this.isLoading = loading;
    
    const btnText = this.translateBtn?.querySelector('.btn__text');
    const btnLoading = this.translateBtn?.querySelector('.btn__loading');
    
    if (loading) {
      btnText?.classList.add('hidden');
      btnLoading?.classList.remove('hidden');
      this.translateBtn?.setAttribute('disabled', 'true');
      if (this.sourceText) this.sourceText.disabled = true;
    } else {
      btnText?.classList.remove('hidden');
      btnLoading?.classList.add('hidden');
      this.translateBtn?.removeAttribute('disabled');
      if (this.sourceText) this.sourceText.disabled = false;
    }
  }

  private escapeHtml(text: string): string {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
  }

  mount(parent: HTMLElement): void {
    parent.appendChild(this.element);
  }

  destroy(): void {
    this.element.remove();
  }
}
