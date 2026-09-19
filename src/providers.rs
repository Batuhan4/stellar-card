use crate::error::{AppError, ErrorCode};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone)]
pub struct StripeProvider {
    client: reqwest::Client,
    base_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StripeCardholder {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StripeAccount {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StripeCard {
    pub id: String,
    pub status: Option<String>,
    pub last4: Option<String>,
    pub brand: Option<String>,
    pub exp_month: Option<u32>,
    pub exp_year: Option<u32>,
    pub number: Option<String>,
    pub cvc: Option<String>,
}

impl StripeProvider {
    pub fn new(base_url: impl Into<String>, api_key: impl Into<String>) -> Result<Self, AppError> {
        let api_key = api_key.into();
        let mut headers = HeaderMap::new();
        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/x-www-form-urlencoded"),
        );
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", api_key)).map_err(|_| {
                AppError::new(ErrorCode::AuthenticationFailure, "Invalid Stripe API key")
            })?,
        );
        Ok(Self {
            client: reqwest::Client::builder()
                .default_headers(headers)
                .build()?,
            base_url: base_url.into(),
        })
    }

    pub async fn validate_api_key(&self) -> Result<(), AppError> {
        let response = self
            .client
            .get(format!("{}/v1/balance", self.base_url))
            .send()
            .await?;
        let _: Value = parse_stripe_response(response).await?;
        Ok(())
    }

    pub async fn create_cardholder(
        &self,
        name: &str,
        email: &str,
    ) -> Result<StripeCardholder, AppError> {
        let names: Vec<&str> = name.split_whitespace().collect();
        let first_name = names.first().copied().unwrap_or("Demo");
        let last_name = names.get(1).copied().unwrap_or("User");
        let body = [
            ("type", "individual".to_string()),
            ("name", name.to_string()),
            ("email", email.to_string()),
            ("individual[first_name]", first_name.to_string()),
            ("individual[last_name]", last_name.to_string()),
            ("individual[dob][day]", "1".to_string()),
            ("individual[dob][month]", "1".to_string()),
            ("individual[dob][year]", "1990".to_string()),
            ("billing[address][line1]", "510 Townsend Street".to_string()),
            ("billing[address][city]", "San Francisco".to_string()),
            ("billing[address][state]", "CA".to_string()),
            ("billing[address][postal_code]", "94103".to_string()),
            ("billing[address][country]", "US".to_string()),
            (
                "individual[card_issuing][user_terms_acceptance][date]",
                format!("{}", chrono::Utc::now().timestamp()),
            ),
            (
                "individual[card_issuing][user_terms_acceptance][ip]",
                "127.0.0.1".to_string(),
            ),
        ];
        let value = self
            .client
            .post(format!("{}/v1/issuing/cardholders", self.base_url))
            .form(&body)
            .send()
            .await?;
        parse_stripe_response(value).await
    }

    pub async fn validate_key(&self) -> Result<StripeAccount, AppError> {
        let response = self
            .client
            .get(format!("{}/v1/account", self.base_url))
            .send()
            .await?;
        parse_stripe_response(response).await
    }

    pub async fn create_virtual_card(&self, cardholder_id: &str) -> Result<StripeCard, AppError> {
        let body = [
            ("cardholder", cardholder_id.to_string()),
            ("currency", "usd".to_string()),
            ("type", "virtual".to_string()),
            ("status", "active".to_string()),
        ];
        let value = self
            .client
            .post(format!("{}/v1/issuing/cards", self.base_url))
            .form(&body)
            .send()
            .await?;
        parse_stripe_response(value).await
    }

    pub async fn retrieve_card(&self, card_id: &str) -> Result<StripeCard, AppError> {
        let response = self
            .client
            .get(format!("{}/v1/issuing/cards/{}", self.base_url, card_id))
            .query(&[("expand[]", "number"), ("expand[]", "cvc")])
            .send()
            .await?;
        parse_stripe_response(response).await
    }

    pub async fn freeze_card(&self, card_id: &str) -> Result<StripeCard, AppError> {
        let body = [("status", "inactive".to_string())];
        let response = self
            .client
            .post(format!("{}/v1/issuing/cards/{}", self.base_url, card_id))
            .form(&body)
            .send()
            .await?;
        parse_stripe_response(response).await
    }
}

async fn parse_stripe_response<T: for<'de> Deserialize<'de>>(
    response: reqwest::Response,
) -> Result<T, AppError> {
    let status = response.status();
    let text = response.text().await?;
    if !status.is_success() {
        let value: Value = serde_json::from_str(&text).unwrap_or_else(|_| json!({"raw": text}));
        let message = value
            .get("error")
            .and_then(|error| error.get("message"))
            .and_then(Value::as_str)
            .unwrap_or("Stripe request failed")
            .to_string();
        let code = if status.as_u16() == 401 {
            ErrorCode::AuthenticationFailure
        } else if status.as_u16() == 429 {
            ErrorCode::RateLimited
        } else {
            ErrorCode::ExternalService
        };
        return Err(AppError::new(code, message).with_details(value));
    }
    Ok(serde_json::from_str(&text)?)
}
