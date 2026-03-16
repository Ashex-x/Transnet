/**
 * Main Transnet translation page.
 */

import { PageShell } from '../shared/page-shell';
import { t } from '../shared/language';

interface TranslateApiSuccess {
  success: true;
  data: {
    translation_id: string;
    text: string;
    source_lang: string;
    target_lang: string;
    input_type: 'word' | 'phrase' | 'sentence' | 'paragraph' | 'essay';
    provider: string;
    model: string;
    user_id?: string;
    translation: TranslationData;
  };
}

interface TranslateApiError {
  success: false;
  error: {
    code?: string;
    message?: string;
  };
}

interface WordMeaning {
  source: string;
  translation: string;
}

interface Explain {
  meaning: string;
  story: string;
  when_to_use: string;
  how_to_use: string;
  context: string;
  lexical_analysis: {
    root?: string;
    structure?: string;
    idiomatic?: boolean;
    related_phrases?: string[];
  };
}

interface RelatedWord {
  word: string;
  type: string;
  similarity: number;
}

interface Relationships {
  related_words?: RelatedWord[];
  related_phrases?: Array<{
    phrase: string;
    type: string;
    similarity: number;
  }>;
  related_concepts?: string[];
  by_pos?: {
    nouns?: string[];
    verbs?: string[];
    adjectives?: string[];
  };
}

interface TranslationWordBasic {
  headword: string;
  part_of_speech: string;
  phonetic: string;
  translations: string[];
  synonyms: string[];
  antonyms: string[];
  examples: WordMeaning[];
}

interface TranslationWordExplain extends TranslationWordBasic {
  explain: Explain;
}

interface TranslationWordFullAnalysis extends TranslationWordExplain {
  relationships: Relationships;
}

interface TranslationPhraseBasic {
  phrase: string;
  headword: string;
  part_of_speech: string;
  translations: string[];
  examples: WordMeaning[];
}

interface TranslationPhraseExplain extends TranslationPhraseBasic {
  explain: Explain;
}

interface TranslationPhraseFullAnalysis extends TranslationPhraseExplain {
  relationships: Relationships;
}

interface TranslationSentenceBasic {
  tone: string;
  rephrasing: string;
}

interface TranslationSentenceExplain extends TranslationSentenceBasic {
  explain: {
    meaning: string;
    usage: string;
    context: string;
  };
}

interface TranslationParagraphEssayBasic {
  text: string;
  translation: string;
}

type TranslationData =
  | TranslationWordBasic
  | TranslationWordExplain
  | TranslationWordFullAnalysis
  | TranslationPhraseBasic
  | TranslationPhraseExplain
  | TranslationPhraseFullAnalysis
  | TranslationSentenceBasic
  | TranslationSentenceExplain
  | TranslationParagraphEssayBasic;

export class Transnet {
  private container: HTMLElement;
  private shell: PageShell | null = null;
  private mainElement: HTMLElement | null = null;
  private apiUrl: string;

  constructor(container: HTMLElement) {
    this.container = container;
    this.apiUrl = '/api/transnet';
  }

  /**
   * Render the Transnet translation UI using PageShell.
   */
  async render(): Promise<void> {
    this.container.innerHTML = '';

    // Create PageShell with header and footer
    this.shell = new PageShell(this.container, {
      requiresAuth: false,
      showFooter: true,
      showTransnetNav: true,
      mainClassName: 'transnet-page',
    });

    this.mainElement = this.shell.mount();

    // Create translation UI content using innerHTML
    this.mainElement.innerHTML = `
      <main class="transnet-stage">
        <div class="transnet-stage__content">
          <!-- Translation Section -->
          <div class="transnet-translation">
            <!-- Input Section -->
            <div class="transnet-translation__section">
              <div class="transnet-section__header">
                <label>${t('transnetSource')}</label>
              </div>
              <textarea class="transnet-source-text" placeholder="${t('transnetSourcePlaceholder')}"></textarea>
              <div class="transnet-image-upload-area">
                <input type="file" accept="image/*" class="transnet-image-input">
                <div class="transnet-upload-placeholder">
                  <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path>
                    <polyline points="17 8 12 3 7 8"></polyline>
                    <line x1="12" y1="3" x2="12" y2="15"></line>
                  </svg>
                  <span>${t('transnetUploadImage')}</span>
                </div>
              </div>
              <img class="transnet-image-preview" alt="Preview">
            </div>

            <!-- Output Section -->
            <div class="transnet-translation__section">
              <div class="transnet-section__header">
                <label>${t('transnetTarget')}</label>
              </div>
              <textarea class="transnet-target-text" placeholder="${t('transnetTargetPlaceholder')}" readonly></textarea>
            </div>
          </div>

          <!-- Translation Config Section -->
          <div class="transnet-config">
            <div class="transnet-config__controls">
              <!-- Language selectors row -->
              <div class="transnet-config__row">
                <select class="transnet-source-lang"></select>
                <button class="transnet-swap-button">
                  <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M11 7l-8 5 8 5"></path>
                    <path d="M13 7l8 5-8 5"></path>
                  </svg>
                </button>
                <select class="transnet-target-lang"></select>
              </div>

              <!-- Output type selector row -->
              <div class="transnet-config__row">
                <label>${t('transnetOutputType')}</label>
                <select class="transnet-output-type">
                  <option value="basic" selected>Basic</option>
                  <option value="explain">Explain</option>
                  <option value="full_analysis">Full</option>
                </select>
              </div>

              <!-- Input type selector row -->
              <div class="transnet-config__row">
                <label>${t('transnetInputType')}</label>
                <select class="transnet-input-type">
                  <option value="text" selected>${t('transnetText')}</option>
                  <option value="image">${t('transnetImage')}</option>
                </select>
              </div>

              <!-- Translate button row -->
              <div class="transnet-config__row">
                <button class="transnet-translate-button">${t('transnetTranslate')}</button>
              </div>
            </div>
          </div>

          <!-- Status Section -->
          <p class="transnet-status" data-state="idle"></p>

          <!-- Extra Output Section -->
          <div class="transnet-extra-output">
            <div class="transnet-extra-output__title">${t('transnetExtraOutput')}</div>
            <div class="transnet-extra-output__content"></div>
          </div>
        </div>
      </main>
    `;

    // Populate language selectors
    const sourceLangSelect = this.mainElement.querySelector('.transnet-source-lang') as HTMLSelectElement;
    const targetLangSelect = this.mainElement.querySelector('.transnet-target-lang') as HTMLSelectElement;
    if (sourceLangSelect && targetLangSelect) {
      this.populateLanguageOptions(sourceLangSelect, 'en');
      this.populateLanguageOptions(targetLangSelect, 'es');
    }

    // Attach event listeners
    const sourceText = this.mainElement.querySelector('.transnet-source-text') as HTMLTextAreaElement;
    const imageUploadArea = this.mainElement.querySelector('.transnet-image-upload-area') as HTMLElement;
    const imageInput = this.mainElement.querySelector('.transnet-image-input') as HTMLInputElement;
    const swapButton = this.mainElement.querySelector('.transnet-swap-button') as HTMLButtonElement;
    const inputTypeSelect = this.mainElement.querySelector('.transnet-input-type') as HTMLSelectElement;
    const translateButton = this.mainElement.querySelector('.transnet-translate-button') as HTMLButtonElement;

    if (sourceText) {
      sourceText.addEventListener('keydown', (event: KeyboardEvent) => {
        if ((event.ctrlKey || event.metaKey) && event.key === 'Enter') {
          event.preventDefault();
          void this.onTranslate();
        }
      });
    }

    if (imageUploadArea && imageInput) {
      imageUploadArea.addEventListener('click', () => {
        imageInput.click();
      });

      imageInput.addEventListener('change', (event: Event) => {
        const target = event.target as HTMLInputElement;
        const file = target.files?.[0];
        if (file) {
          void this.onImageUpload(file);
        }
      });
    }

    if (swapButton && sourceLangSelect && targetLangSelect) {
      swapButton.addEventListener('click', () => {
        const currentSource = sourceLangSelect.value;
        sourceLangSelect.value = targetLangSelect.value;
        targetLangSelect.value = currentSource;
      });
    }

    if (inputTypeSelect) {
      inputTypeSelect.addEventListener('change', (event: Event) => {
        const target = event.target as HTMLSelectElement;
        const type = target.value as 'text' | 'image';
        this.onInputChangeType(type);
      });
    }

    if (translateButton) {
      translateButton.addEventListener('click', () => {
        void this.onTranslate();
      });
    }
  }

  /**
   * Populate a language selector and set its default value.
   */
  private populateLanguageOptions(select: HTMLSelectElement, defaultValue: string): void {
    const languages: Array<[string, string]> = [
      ['en', 'English'],
      ['es', 'Spanish'],
      ['zh', 'Chinese'],
      ['ja', 'Japanese'],
      ['fr', 'French'],
      ['de', 'German'],
      ['ko', 'Korean'],
    ];

    for (const [value, label] of languages) {
      const option = document.createElement('option');
      option.value = value;
      option.textContent = label;
      option.selected = value === defaultValue;
      select.appendChild(option);
    }
  }

  /**
   * Handle image upload and display preview.
   */
  private async onImageUpload(file: File): Promise<void> {
    const imagePreview = this.mainElement?.querySelector('.transnet-image-preview') as HTMLImageElement;
    const imageUploadArea = this.mainElement?.querySelector('.transnet-image-upload-area') as HTMLElement;

    if (!imagePreview || !imageUploadArea) {
      return;
    }

    // Display image preview
    const reader = new FileReader();
    reader.onload = (event) => {
      if (event.target?.result) {
        imagePreview.src = event.target.result as string;
        imagePreview.classList.add('visible');
        imageUploadArea.style.display = 'none';
      }
    };
    reader.readAsDataURL(file);

    // Image processor logic placeholder - no business logic implemented
    // Future implementation will process the uploaded image here
  }

  /**
   * Handle input type switching between text and image.
   */
  private onInputChangeType(type: 'text' | 'image'): void {
    const sourceText = this.mainElement?.querySelector('.transnet-source-text') as HTMLTextAreaElement;
    const imageUploadArea = this.mainElement?.querySelector('.transnet-image-upload-area') as HTMLElement;
    const imagePreview = this.mainElement?.querySelector('.transnet-image-preview') as HTMLImageElement;

    if (!sourceText || !imageUploadArea || !imagePreview) {
      return;
    }

    if (type === 'image') {
      // Show image upload/preview, hide textarea
      sourceText.style.display = 'none';
      imageUploadArea.classList.add('visible');
      imageUploadArea.style.display = 'block';
    } else {
      // Show textarea, hide image upload/preview
      sourceText.style.display = 'block';
      imageUploadArea.classList.remove('visible');
      imageUploadArea.style.display = 'none';
      imagePreview.classList.remove('visible');
      imagePreview.src = '';
    }
  }

  /**
   * Send the translate request and update the UI with the result or error.
   */
  private async onTranslate(): Promise<void> {
    const sourceText = this.mainElement?.querySelector('.transnet-source-text') as HTMLTextAreaElement;
    const targetText = this.mainElement?.querySelector('.transnet-target-text') as HTMLTextAreaElement;
    const sourceLang = this.mainElement?.querySelector('.transnet-source-lang') as HTMLSelectElement;
    const targetLang = this.mainElement?.querySelector('.transnet-target-lang') as HTMLSelectElement;
    const outputType = this.mainElement?.querySelector('.transnet-output-type') as HTMLSelectElement;
    const translateButton = this.mainElement?.querySelector('.transnet-translate-button') as HTMLButtonElement;
    const status = this.mainElement?.querySelector('.transnet-status') as HTMLParagraphElement;

    if (!sourceText || !targetText || !sourceLang || !targetLang || !outputType || !translateButton || !status) {
      return;
    }

    const text = sourceText.value.trim();
    if (!text) {
      targetText.value = '';
      status.textContent = t('transnetEnterTextBeforeTranslating');
      status.dataset.state = 'error';
      return;
    }

    translateButton.disabled = true;
    translateButton.textContent = t('transnetTranslating');
    status.textContent = t('transnetContactingBackend');
    status.dataset.state = 'loading';

    try {
      const headers: HeadersInit = {
        'Content-Type': 'application/json',
      };

      const authToken = this.getAuthToken();
      if (authToken) {
        headers['Authorization'] = `Bearer ${authToken}`;
      }

      const response = await fetch(`${this.apiUrl}/translate`, {
        method: 'POST',
        headers,
        body: JSON.stringify({
          text,
          source_lang: sourceLang.value,
          target_lang: targetLang.value,
          mode: outputType.value,
        }),
      });

      const payload: TranslateApiSuccess | TranslateApiError = await response.json();

      if (!response.ok || !('success' in payload) || !payload.success || !payload.data) {
        const message = 'error' in payload ? payload.error.message ?? t('transnetTranslationFailed') : t('transnetTranslationFailed');
        throw new Error(message);
      }

      this.renderTranslation(payload.data.translation, targetText);
      this.displayExtraOutput(payload.data);

      status.textContent = `Translated as ${payload.data.input_type} via ${payload.data.model}.`;
      status.dataset.state = 'success';
    } catch (error) {
      targetText.value = '';
      status.textContent =
        error instanceof Error ? error.message : t('transnetUnableToReach');
      status.dataset.state = 'error';
    } finally {
      translateButton.disabled = false;
      translateButton.textContent = 'Translate';
    }
  }


  /**
   * Get JWT token from localStorage if available.
   */
  private getAuthToken(): string | null {
    try {
      const authData = localStorage.getItem('auth');
      if (authData) {
        const parsed = JSON.parse(authData);
        return parsed.access_token || null;
      }
    } catch {
      return null;
    }
    return null;
  }

  /**
   * Render main translation text in target textarea based on translation type.
   */
  private renderTranslation(translationData: TranslationData, targetElement: HTMLTextAreaElement): void {
    if ('translations' in translationData) {
      targetElement.value = translationData.translations.join(', ');
    } else if ('rephrasing' in translationData) {
      targetElement.value = translationData.rephrasing;
    } else if ('translation' in translationData && 'text' in translationData) {
      targetElement.value = translationData.translation;
    } else {
      targetElement.value = '';
    }
  }

  /**
   * Display extra output details in the extra output section.
   */
  private displayExtraOutput(data: TranslateApiSuccess['data']): void {
    const extraOutputContent = this.mainElement?.querySelector('.transnet-extra-output__content');
    if (!extraOutputContent) {
      return;
    }

    const { translation, input_type } = data;
    let html = '';

    if (input_type === 'word') {
      html = this.renderWordExtra(
        translation as TranslationWordBasic | TranslationWordExplain | TranslationWordFullAnalysis,
      );
    } else if (input_type === 'phrase') {
      html = this.renderPhraseExtra(
        translation as TranslationPhraseBasic | TranslationPhraseExplain | TranslationPhraseFullAnalysis,
      );
    } else if (input_type === 'sentence') {
      html = this.renderSentenceExtra(translation as TranslationSentenceBasic | TranslationSentenceExplain);
    } else if (input_type === 'paragraph' || input_type === 'essay') {
      html = this.renderParagraphEssayExtra(translation as TranslationParagraphEssayBasic);
    }

    extraOutputContent.innerHTML = html;
  }

  /**
   * Render extra output for word translations.
   */
  private renderWordExtra(
    translation: TranslationWordBasic | TranslationWordExplain | TranslationWordFullAnalysis,
  ): string {
    const isFullAnalysis = 'relationships' in translation;
    const hasExplain = 'explain' in translation;

    let html = `
      <div class="transnet-extra-output__section">
        <h4>Word Information</h4>
        <p><strong>Word:</strong> ${translation.headword}</p>
        <p><strong>Part of Speech:</strong> ${translation.part_of_speech}</p>
        <p><strong>Phonetic:</strong> ${translation.phonetic}</p>
      </div>
    `;

    if (translation.synonyms && translation.synonyms.length > 0) {
      html += `
        <div class="transnet-extra-output__section">
          <h4>Synonyms</h4>
          <p>${translation.synonyms.join(', ')}</p>
        </div>
      `;
    }

    if (translation.antonyms && translation.antonyms.length > 0) {
      html += `
        <div class="transnet-extra-output__section">
          <h4>Antonyms</h4>
          <p>${translation.antonyms.join(', ')}</p>
        </div>
      `;
    }

    if (translation.examples && translation.examples.length > 0) {
      html += `
        <div class="transnet-extra-output__section">
          <h4>Examples</h4>
          ${translation.examples.map(
            (ex) => `
              <div class="transnet-extra-output__example">
                <p class="source">${ex.source}</p>
                <p class="translation">${ex.translation}</p>
              </div>
            `,
          ).join('')}
        </div>
      `;
    }

    if (hasExplain) {
      html += `
        <div class="transnet-extra-output__section">
          <h4>Explanation</h4>
          <p><strong>Meaning:</strong> ${translation.explain.meaning}</p>
          <p><strong>Story:</strong> ${translation.explain.story}</p>
          <p><strong>When to use:</strong> ${translation.explain.when_to_use}</p>
          <p><strong>How to use:</strong> ${translation.explain.how_to_use}</p>
          <p><strong>Context:</strong> ${translation.explain.context}</p>
          ${translation.explain.lexical_analysis.root ? `<p><strong>Root:</strong> ${translation.explain.lexical_analysis.root}</p>` : ''}
          ${translation.explain.lexical_analysis.structure ? `<p><strong>Structure:</strong> ${translation.explain.lexical_analysis.structure}</p>` : ''}
          ${translation.explain.lexical_analysis.idiomatic ? `<p><strong>Idiomatic:</strong> Yes</p>` : ''}
        </div>
      `;
    }

    if (isFullAnalysis) {
      const rels = translation.relationships;
      html += `
        <div class="transnet-extra-output__section">
          <h4>Relationships</h4>
          ${rels.related_words && rels.related_words.length > 0 ? `
            <h5>Related Words</h5>
            <ul>
              ${rels.related_words.map((rw) => `<li>${rw.word} (${rw.type}, similarity: ${rw.similarity})</li>`).join('')}
            </ul>
          ` : ''}
          ${rels.by_pos ? `
            <h5>By Part of Speech</h5>
            ${rels.by_pos.nouns ? `<p><strong>Nouns:</strong> ${rels.by_pos.nouns.join(', ')}</p>` : ''}
            ${rels.by_pos.verbs ? `<p><strong>Verbs:</strong> ${rels.by_pos.verbs.join(', ')}</p>` : ''}
            ${rels.by_pos.adjectives ? `<p><strong>Adjectives:</strong> ${rels.by_pos.adjectives.join(', ')}</p>` : ''}
          ` : ''}
        </div>
      `;
    }

    return html;
  }

  /**
   * Render extra output for phrase translations.
   */
  private renderPhraseExtra(
    translation: TranslationPhraseBasic | TranslationPhraseExplain | TranslationPhraseFullAnalysis,
  ): string {
    const isFullAnalysis = 'relationships' in translation;
    const hasExplain = 'explain' in translation;

    let html = `
      <div class="transnet-extra-output__section">
        <h4>Phrase Information</h4>
        <p><strong>Phrase:</strong> ${translation.phrase}</p>
        <p><strong>Part of Speech:</strong> ${translation.part_of_speech}</p>
      </div>
    `;

    if (translation.examples && translation.examples.length > 0) {
      html += `
        <div class="transnet-extra-output__section">
          <h4>Examples</h4>
          ${translation.examples.map(
            (ex) => `
              <div class="transnet-extra-output__example">
                <p class="source">${ex.source}</p>
                <p class="translation">${ex.translation}</p>
              </div>
            `,
          ).join('')}
        </div>
      `;
    }

    if (hasExplain) {
      html += `
        <div class="transnet-extra-output__section">
          <h4>Explanation</h4>
          <p><strong>Meaning:</strong> ${translation.explain.meaning}</p>
          <p><strong>Story:</strong> ${translation.explain.story}</p>
          <p><strong>When to use:</strong> ${translation.explain.when_to_use}</p>
          <p><strong>How to use:</strong> ${translation.explain.how_to_use}</p>
          <p><strong>Context:</strong> ${translation.explain.context}</p>
          ${translation.explain.lexical_analysis.structure ? `<p><strong>Structure:</strong> ${translation.explain.lexical_analysis.structure}</p>` : ''}
          ${translation.explain.lexical_analysis.idiomatic ? `<p><strong>Idiomatic:</strong> Yes</p>` : ''}
          ${translation.explain.lexical_analysis.related_phrases && translation.explain.lexical_analysis.related_phrases.length > 0 ? `
            <p><strong>Related Phrases:</strong> ${translation.explain.lexical_analysis.related_phrases.join(', ')}</p>
          ` : ''}
        </div>
      `;
    }

    if (isFullAnalysis) {
      const rels = translation.relationships;
      html += `
        <div class="transnet-extra-output__section">
          <h4>Relationships</h4>
          ${rels.related_phrases && rels.related_phrases.length > 0 ? `
            <h5>Related Phrases</h5>
            <ul>
              ${rels.related_phrases.map((rp) => `<li>${rp.phrase} (${rp.type}, similarity: ${rp.similarity})</li>`).join('')}
            </ul>
          ` : ''}
          ${rels.related_concepts && rels.related_concepts.length > 0 ? `
            <h5>Related Concepts</h5>
            <p>${rels.related_concepts.join(', ')}</p>
          ` : ''}
        </div>
      `;
    }

    return html;
  }

  /**
   * Render extra output for sentence translations.
   */
  private renderSentenceExtra(
    translation: TranslationSentenceBasic | TranslationSentenceExplain,
  ): string {
    let html = `
      <div class="transnet-extra-output__section">
        <h4>Sentence Information</h4>
        <p><strong>Tone:</strong> ${translation.tone}</p>
      </div>
    `;

    if ('explain' in translation) {
      html += `
        <div class="transnet-extra-output__section">
          <h4>Explanation</h4>
          <p><strong>Meaning:</strong> ${translation.explain.meaning}</p>
          <p><strong>Usage:</strong> ${translation.explain.usage}</p>
          <p><strong>Context:</strong> ${translation.explain.context}</p>
        </div>
      `;
    }

    return html;
  }

  /**
   * Render extra output for paragraph/essay translations.
   */
  private renderParagraphEssayExtra(translation: TranslationParagraphEssayBasic): string {
    return `
      <div class="transnet-extra-output__section">
        <h4>Translation Comparison</h4>
        <div class="transnet-extra-output__comparison">
          <div class="transnet-extra-output__comparison-side">
            <h5>Original</h5>
            <p>${translation.text}</p>
          </div>
          <div class="transnet-extra-output__comparison-side">
            <h5>Translation</h5>
            <p>${translation.translation}</p>
          </div>
        </div>
      </div>
    `;
  }

  /**
   * Remove the page from the DOM and clean up resources.
   */
  destroy(): void {
    this.shell?.destroy();
    this.shell = null;
    this.mainElement = null;
  }
}
