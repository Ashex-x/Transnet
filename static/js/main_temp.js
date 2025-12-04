// Set up API endpoint details (Replace these with your actual values if needed)
// If you are using the Gemini API directly from the client, set the model and API URL here.

// Utility to convert Base64 to ArrayBuffer (for internal API handling if needed)
function base64ToArrayBuffer(base64) {
  const binaryString = atob(base64);
  const len = binaryString.length;
  const bytes = new Uint8Array(len);
  for (let i = 0; i < len; i++) {
    bytes[i] = binaryString.charCodeAt(i);
  }
  return bytes.buffer;
}

/**
 * The core function to process user input and call the backend/API.
 */
async function transInput() {
  const kInputElement = document.getElementById('input-text');
  const kOutputElement = document.getElementById('output-text');
  const kLoadingIndicator = document.getElementById('output-loading-indicator');
  const kTranslateButton = document.getElementById('translate-button');

  const kUserInput = kInputElement.value.trim();

  if (!kUserInput) {
    kOutputElement.value = "Please enter some text to translate.";
    return;
  }

  // UI State: Disable button and show loading
  kTranslateButton.disabled = true;
  kLoadingIndicator.classList.remove('hidden');
  kOutputElement.value = ""; // Clear previous output

  try {
    // Send Chinese text to backend
    const response = await fetch('http://localhost:5000/api/input-text', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        text: kUserInput,
        timestamp: new Date().toISOString(),
        input_length: kUserInput.length,
        source: 'web_form'
      })
    });

    if (!response.ok) {
      throw new Error(`HTTP Error: ${response.status}`);
    }

  } catch (error) {
    console.error('Submit fail: ', error);
    kOutputElement.textContent = `Submit fail: ${error.mess}`;
  } finally {
    // UI State: Re-enable button and hide loading
    kTranslateButton.disabled = false;
    kLoadingIndicator.classList.add('hidden');
  }
}