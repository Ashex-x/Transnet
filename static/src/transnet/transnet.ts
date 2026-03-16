/**
 * Main Transnet translation page.
 */

import { PageShell } from '../shared/page-shell';

interface TranslateApiSuccess {
  success: true;
  data: {
    translation: string;
    source_lang: string;
    target_lang: string;
    input_type: string;
    provider: string;
    model: string;
  };
}

interface TranslateApiError {
  success: false;
  error?: {
    code?: string;
    message?: string;
  };
}

export class Transnet {
  private container: HTMLElement;
  private readonly apiUrl: string;
  private shell: PageShell | null = null;
  private mainElement: HTMLElement | null = null;

  constructor(container: HTMLElement) {
    this.container = container;
    this.apiUrl = '/transnet/api';
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
                <label>Source</label>
              </div>
              <textarea class="transnet-source-text" placeholder="Enter text to translate... Press Ctrl+Enter to translate quickly."></textarea>
              <div class="transnet-image-upload-area">
                <input type="file" accept="image/*" class="transnet-image-input">
                <div class="transnet-upload-placeholder">
                  <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path>
                    <polyline points="17 8 12 3 7 8"></polyline>
                    <line x1="12" y1="3" x2="12" y2="15"></line>
                  </svg>
                  <span>Click to upload an image</span>
                </div>
              </div>
              <img class="transnet-image-preview" alt="Preview">
            </div>

            <!-- Output Section -->
            <div class="transnet-translation__section">
              <div class="transnet-section__header">
                <label>Target</label>
              </div>
              <textarea class="transnet-target-text" placeholder="Translation will appear here..." readonly></textarea>
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
                <label>Output Type:</label>
                <select class="transnet-output-type">
                  <option value="basic" selected>Basic</option>
                  <option value="explain">Explain</option>
                  <option value="full_analysis">Full</option>
                </select>
              </div>

              <!-- Input type selector row -->
              <div class="transnet-config__row">
                <label>Input Type:</label>
                <select class="transnet-input-type">
                  <option value="text" selected>Text</option>
                  <option value="image">Image</option>
                </select>
              </div>

              <!-- Translate button row -->
              <div class="transnet-config__row">
                <button class="transnet-translate-button">Translate</button>
              </div>
            </div>
          </div>

          <!-- Status Section -->
          <p class="transnet-status" data-state="idle"></p>

          <!-- Extra Output Section -->
          <div class="transnet-extra-output">
            <div class="transnet-extra-output__title">Extra Output</div>
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
      status.textContent = 'Enter text before translating.';
      status.dataset.state = 'error';
      return;
    }

    translateButton.disabled = true;
    translateButton.textContent = 'Translating...';
    status.textContent = 'Contacting Transnet backend...';
    status.dataset.state = 'loading';

    try {
      const response = await fetch(`${this.apiUrl}/translate`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          text,
          source_lang: sourceLang.value,
          target_lang: targetLang.value,
          mode: outputType.value,
          input_type: 'auto',
        }),
      });

      const payload = (await response.json()) as TranslateApiSuccess | TranslateApiError;
      if (!response.ok || !payload.success) {
        const message =
          payload.success === false
            ? payload.error?.message ?? 'Translation failed.'
            : `Request failed with status ${response.status}.`;
        throw new Error(message);
      }

      targetText.value = payload.data.translation;
      status.textContent = `Translated as ${payload.data.input_type} via ${payload.data.model}.`;
      status.dataset.state = 'success';
    } catch (error) {
      targetText.value = '';
      status.textContent =
        error instanceof Error ? error.message : 'Unable to reach the Transnet backend.';
      status.dataset.state = 'error';
    } finally {
      translateButton.disabled = false;
      translateButton.textContent = 'Translate';
    }
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
