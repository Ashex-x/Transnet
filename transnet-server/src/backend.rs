//! HTTP client for the translation backend.

use reqwest::Client;
use transnet::types::{
  InputType as BackendInputType, TranslateResponse as BackendTranslateResponse,
};

use crate::{
  ErrorInfo, ErrorResponse, HealthData, InputType, SuccessResponse, TranslateRequest,
  TranslateResponse, TransnetError,
};

/// Sends gateway translation and health requests to the backend service.
#[derive(Clone, Debug)]
pub struct BackendClient {
  base_url: String,
  client: Client,
}

impl BackendClient {
  /// Creates a backend client and removes trailing slashes from `base_url`.
  pub fn new(base_url: impl Into<String>) -> Self {
    Self {
      base_url: base_url.into().trim_end_matches('/').to_string(),
      client: Client::new(),
    }
  }

  /// Fetches the backend health payload.
  ///
  /// # Errors
  ///
  /// Returns [`TransnetError::Backend`] when the request, status, or response body is invalid.
  pub async fn health(&self) -> Result<HealthData, TransnetError> {
    let url = format!("{}/health", self.base_url);
    let response = self
      .client
      .get(url)
      .send()
      .await
      .map_err(|error| TransnetError::Backend(format!("health check failed: {error}")))?;

    if !response.status().is_success() {
      return Err(TransnetError::Backend(format!(
        "health check returned status {}",
        response.status()
      )));
    }

    let body: SuccessResponse<HealthData> = response.json().await.map_err(|error| {
      TransnetError::Backend(format!("failed to parse health response: {error}"))
    })?;
    Ok(body.data)
  }

  /// Translates text through the backend and maps its response into the gateway contract.
  ///
  /// # Errors
  ///
  /// Returns [`TransnetError::Validation`] for backend status 422 and
  /// [`TransnetError::Backend`] for transport, parsing, or other status failures.
  pub async fn translate(
    &self,
    request: TranslateRequest,
  ) -> Result<TranslateResponse, TransnetError> {
    let url = format!("{}/translate", self.base_url);
    let response = self
      .client
      .post(url)
      .json(&request)
      .send()
      .await
      .map_err(|error| TransnetError::Backend(format!("translation request failed: {error}")))?;
    let status = response.status();

    if !status.is_success() {
      let error_body: ErrorResponse = response.json().await.unwrap_or_else(|_| ErrorResponse {
        success: false,
        error: ErrorInfo {
          code: "UNKNOWN".to_string(),
          message: "failed to parse error response".to_string(),
        },
      });
      let message = format!("backend returned {status}: {}", error_body.error.message);
      return if status.as_u16() == 422 {
        Err(TransnetError::Validation(message))
      } else {
        Err(TransnetError::Backend(message))
      };
    }

    let body: SuccessResponse<BackendTranslateResponse> =
      response.json().await.map_err(|error| {
        TransnetError::Backend(format!("failed to parse translation response: {error}"))
      })?;
    Ok(map_translation(body.data))
  }
}

fn map_translation(response: BackendTranslateResponse) -> TranslateResponse {
  TranslateResponse {
    translation_id: response.translation_id,
    text: response.text,
    source_lang: response.source_lang,
    target_lang: response.target_lang,
    input_type: match response.input_type {
      BackendInputType::Auto => InputType::Auto,
      BackendInputType::Word => InputType::Word,
      BackendInputType::Phrase => InputType::Phrase,
      BackendInputType::Sentence => InputType::Sentence,
      BackendInputType::Paragraph => InputType::Paragraph,
      BackendInputType::Essay => InputType::Essay,
    },
    user_id: None,
    translation: response.translation,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_new_with_trailing_slashes_normalizes_base_url() {
    let client = BackendClient::new("http://localhost:35792///");
    assert_eq!(client.base_url, "http://localhost:35792");
  }
}
