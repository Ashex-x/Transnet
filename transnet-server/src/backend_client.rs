use reqwest::Client;

use crate::{HealthData, TranslateRequest, TranslateResponse, TransnetError};

pub struct BackendClient {
  base_url: String,
  client: Client,
}

impl BackendClient {
  pub fn new(base_url: String) -> Self {
    Self {
      base_url: base_url.trim_end_matches('/').to_string(),
      client: Client::new(),
    }
  }

  pub async fn health(&self) -> Result<HealthData, TransnetError> {
    let url = format!("{}/health", self.base_url);
    let response = self
      .client
      .get(&url)
      .send()
      .await
      .map_err(|e| TransnetError::Backend(format!("health check failed: {e}")))?;

    if !response.status().is_success() {
      let status = response.status();
      return Err(TransnetError::Backend(format!(
        "health check returned status {status}"
      )));
    }

    let body: crate::SuccessResponse<HealthData> = response
      .json()
      .await
      .map_err(|e| TransnetError::Backend(format!("failed to parse health response: {e}")))?;

    Ok(body.data)
  }

  pub async fn translate(&self, request: TranslateRequest) -> Result<TranslateResponse, TransnetError> {
    let url = format!("{}/translate", self.base_url);
    let response = self
      .client
      .post(&url)
      .json(&request)
      .send()
      .await
      .map_err(|e| TransnetError::Backend(format!("translation request failed: {e}")))?;

    let status = response.status();

    if !status.is_success() {
      let error_body: crate::ErrorResponse = response
        .json()
        .await
        .unwrap_or_else(|_| crate::ErrorResponse {
          success: false,
          error: crate::ErrorInfo {
            code: "UNKNOWN".to_string(),
            message: "failed to parse error response".to_string(),
          },
        });

      let message = format!(
        "backend returned {status}: {}",
        error_body.error.message
      );

      return match status.as_u16() {
        422 => Err(TransnetError::Validation(message)),
        _ => Err(TransnetError::Backend(message)),
      };
    }

    let body: crate::SuccessResponse<TranslateResponse> = response
      .json()
      .await
      .map_err(|e| TransnetError::Backend(format!("failed to parse translation response: {e}")))?;

    Ok(body.data)
  }
}
