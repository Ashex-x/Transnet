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

    // Create translation UI content
    const translationContainer = document.createElement('div');
    translationContainer.className = 'translation-container';

    const inputSection = document.createElement('div');
    inputSection.className = 'translation-section';

    const inputHeader = document.createElement('div');
    inputHeader.className = 'section-header';

    const sourceControls = document.createElement('div');
    sourceControls.className = 'translation-controls';

    const sourceLangSelect = document.createElement('select');
    sourceLangSelect.id = 'source-lang';
    this.populateLanguageOptions(sourceLangSelect, 'en');

    const targetLangSelect = document.createElement('select');
    targetLangSelect.id = 'target-lang';
    this.populateLanguageOptions(targetLangSelect, 'es');

    const swapButton = document.createElement('button');
    swapButton.className = 'swap-button';
    swapButton.textContent = 'Swap';
    swapButton.addEventListener('click', () => {
      const currentSource = sourceLangSelect.value;
      sourceLangSelect.value = targetLangSelect.value;
      targetLangSelect.value = currentSource;
    });

    sourceControls.appendChild(sourceLangSelect);
    sourceControls.appendChild(swapButton);
    sourceControls.appendChild(targetLangSelect);

    const sourceLabel = document.createElement('label');
    sourceLabel.textContent = 'Source';

    const inputTextArea = document.createElement('textarea');
    inputTextArea.id = 'source-text';
    inputTextArea.placeholder = 'Enter text to translate...';
    inputTextArea.addEventListener('keydown', (event: KeyboardEvent) => {
      if ((event.ctrlKey || event.metaKey) && event.key === 'Enter') {
        event.preventDefault();
        void this.onTranslate();
      }
    });

    inputHeader.appendChild(sourceControls);
    inputSection.appendChild(inputHeader);
    inputSection.appendChild(sourceLabel);
    inputSection.appendChild(inputTextArea);

    const actionSection = document.createElement('div');
    actionSection.className = 'action-section';

    const translateButton = document.createElement('button');
    translateButton.className = 'translate-button';
    translateButton.textContent = 'Translate';
    translateButton.id = 'translate-button';
    translateButton.addEventListener('click', () => {
      void this.onTranslate();
    });

    const helperText = document.createElement('p');
    helperText.className = 'action-hint';
    helperText.textContent = 'Press Ctrl+Enter to translate quickly.';

    actionSection.appendChild(translateButton);
    actionSection.appendChild(helperText);

    const outputSection = document.createElement('div');
    outputSection.className = 'translation-section';

    const outputHeader = document.createElement('div');
    outputHeader.className = 'section-header';

    const targetLabel = document.createElement('label');
    targetLabel.textContent = 'Target';

    const outputTextArea = document.createElement('textarea');
    outputTextArea.id = 'target-text';
    outputTextArea.placeholder = 'Translation will appear here...';
    outputTextArea.readOnly = true;

    const status = document.createElement('p');
    status.id = 'transnet-status';
    status.className = 'transnet-status';
    status.textContent = 'Ready to translate.';

    outputSection.appendChild(outputHeader);
    outputSection.appendChild(targetLabel);
    outputSection.appendChild(outputTextArea);
    outputSection.appendChild(status);

    translationContainer.appendChild(inputSection);
    translationContainer.appendChild(actionSection);
    translationContainer.appendChild(outputSection);

    this.mainElement.appendChild(translationContainer);
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
   * Send the translate request and update the UI with the result or error.
   */
  private async onTranslate(): Promise<void> {
    const sourceText = document.getElementById('source-text') as HTMLTextAreaElement;
    const targetText = document.getElementById('target-text') as HTMLTextAreaElement;
    const sourceLang = document.getElementById('source-lang') as HTMLSelectElement;
    const targetLang = document.getElementById('target-lang') as HTMLSelectElement;
    const translateButton = document.getElementById('translate-button') as HTMLButtonElement;
    const status = document.getElementById('transnet-status') as HTMLParagraphElement;

    if (!sourceText || !targetText || !sourceLang || !targetLang || !translateButton || !status) {
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
          mode: 'basic',
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
