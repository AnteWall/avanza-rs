use std::borrow::Borrow;
use std::collections::HashMap;

use crate::error::RequestError;
use crate::request::{post, post_response};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct Client {
    pub api_url: String,
    pub user_agent: String,
    x_security_token: String,
    session: String,
    config: Config,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Config {
    pub avanza_username: String,
    pub avanza_password: String,
    pub avanza_totp_secret: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthenticateTOTPResponse {
    authentication_session: String,
    push_subscription_id: String,
    customer_id: String,
    registration_complete: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthenticateResponse {
    two_factor_login: TwoFactorLogin,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TwoFactorLogin {
    method: String,
    transaction_id: String,
}

const MAX_INACTIVE_MINUTES_AS_SECONDS: &str = "3600";

impl Client {
    pub fn new(config: Config) -> Self {
        Self {
            api_url: String::from("https://www.avanza.se"),
            user_agent: String::from("Avanza API client"),
            session: String::new(),
            x_security_token: String::new(),
            config,
        }
    }

    pub fn new_from_env() -> Self {
        let config = envy::from_env::<Config>().expect(
            "please provide AVANZA_USERNAME, AVANZA_PASSWORD and AVANZA_TOTP_SECRET env var",
        );
        Client::new(config)
    }

    pub fn api_url(self, value: String) -> Self {
        Self {
            api_url: value,
            ..self
        }
    }

    pub fn user_agent(self, value: String) -> Self {
        Self {
            user_agent: value,
            ..self
        }
    }

    pub async fn get_response<T: DeserializeOwned>(
        &mut self,
        uri: &str,
    ) -> Result<T, RequestError> {
        let response = reqwest::get(uri).await?;
        let body = response.text().await?;
        Ok(serde_json::from_str::<T>(&body)?)
    }

    pub(crate) fn is_authenticated(&self) -> bool {
        return !self.x_security_token.is_empty() && !self.session.is_empty();
    }

    pub async fn authenticate(&mut self) -> Result<AuthenticateResponse, RequestError> {
        let mut map = HashMap::new();
        let username = self.config.avanza_username.as_str();
        let password = self.config.avanza_password.as_str();
        map.insert("username", username);
        map.insert("password", password);
        map.insert("maxInactiveMinutes", MAX_INACTIVE_MINUTES_AS_SECONDS);

        let uri = format!(
            "{}/_api/authentication/sessions/usercredentials",
            self.api_url
        );

        let response = post_response::<AuthenticateResponse>(&uri, &map).await?;

        if response.two_factor_login.method != "TOTP" {
            return Err(RequestError::UnknownAuthenticationMethod());
        }

        self.authenticate_totp(response.two_factor_login.transaction_id.clone())
            .await?;

        Ok(response)
    }

    async fn authenticate_totp(&mut self, transaction_id: String) -> Result<(), RequestError> {
        let uri = format!("{}/_api/authentication/sessions/totp", self.api_url);
        let mut map = HashMap::new();
        map.insert("totpCode", transaction_id.as_str());
        map.insert("method", "TOTP");

        let response = post(&uri, &map).await?;

        let x_token = String::from_utf8_lossy(
            response
                .borrow()
                .headers()
                .get("x-securitytoken")
                .expect("failed to get x-securitytoken")
                .as_bytes(),
        )
        .to_string();

        let totp_response = response.json::<AuthenticateTOTPResponse>().await?;

        self.x_security_token = x_token;
        self.session = totp_response.authentication_session;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::BorrowMut;

    use super::*;
    use tokio_test::{assert_err, assert_ok};
    use wiremock::matchers::{any, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn correct_default_values() {
        let client = Client::new(Config {
            avanza_username: String::from("user"),
            avanza_password: String::from("pass"),
            avanza_totp_secret: String::from("secret"),
        });

        assert_eq!(client.api_url, String::from("https://www.avanza.se"));
        assert_eq!(client.user_agent, String::from("Avanza API client"));
    }
    #[test]
    fn can_set_api_url() {
        let client = Client::new(Config {
            avanza_username: String::from("user"),
            avanza_password: String::from("pass"),
            avanza_totp_secret: String::from("secret"),
        })
        .api_url(String::from("https://avanza-new.se"));

        assert_eq!(client.api_url, String::from("https://avanza-new.se"));
    }
    #[test]
    fn can_set_user_agent() {
        let client = Client::new(Config {
            avanza_username: String::from("user"),
            avanza_password: String::from("pass"),
            avanza_totp_secret: String::from("secret"),
        })
        .user_agent(String::from("My custom user agent"));

        assert_eq!(client.user_agent, String::from("My custom user agent"));
    }

    #[tokio::test]
    async fn raises_error_on_unknown_authentication_method() {
        let mock_server = MockServer::start().await;

        let responder = ResponseTemplate::new(200).set_body_string(
            String::from("{\"twoFactorLogin\":{\"transactionId\":\"4530ff65-a4d3-4af0-9e9b-22729a6157c9\",\"method\":\"BANKID\"}}")
        );

        Mock::given(method("POST"))
            .and(path("/_api/authentication/sessions/usercredentials"))
            .respond_with(responder)
            .mount(&mock_server)
            .await;

        let mut client = Client::new(Config {
            avanza_username: String::from("user"),
            avanza_password: String::from("pass"),
            avanza_totp_secret: String::from("secret"),
        })
        .api_url(mock_server.uri());

        assert_err!(client.authenticate().await);
    }

    #[tokio::test]
    async fn authentication_success() {
        let mock_server = MockServer::start().await;

        let responder = ResponseTemplate::new(200).set_body_string(
            String::from("{\"twoFactorLogin\":{\"transactionId\":\"4530ff65-a4d3-4af0-9e9b-22729a6157c9\",\"method\":\"TOTP\"}}")
        );

        let mut responder_totp = ResponseTemplate::new(200).set_body_string(
            String::from("{\"authenticationSession\":\"4530ff65-a4d3-4af0-9e9b-22729a6157c9\",\"pushSubscriptionId\":\"54320ff65-a4d3-4af0-9e9b-22729a6157c9\",\"customerId\":\"123232\", \"registrationComplete\": true}")
        );

        responder_totp = responder_totp.append_header("x-securitytoken", "mysecrettoken");

        Mock::given(method("POST"))
            .and(path("/_api/authentication/sessions/usercredentials"))
            .respond_with(responder)
            .mount(&mock_server)
            .await;
        Mock::given(method("POST"))
            .and(path("/_api/authentication/sessions/totp"))
            .respond_with(responder_totp)
            .mount(&mock_server)
            .await;

        let mut client = Client::new(Config {
            avanza_username: String::from("user"),
            avanza_password: String::from("pass"),
            avanza_totp_secret: String::from("secret"),
        })
        .api_url(mock_server.uri());

        assert_ok!(client.authenticate().await);
    }

    #[tokio::test]
    async fn authentication_totp_set_auth() {
        let mock_server = MockServer::start().await;

        let mut responder = ResponseTemplate::new(200).set_body_string(
            String::from("{\"authenticationSession\":\"4530ff65-a4d3-4af0-9e9b-22729a6157c9\",\"pushSubscriptionId\":\"54320ff65-a4d3-4af0-9e9b-22729a6157c9\",\"customerId\":\"123232\", \"registrationComplete\": true}")
        );

        responder = responder.append_header("x-securitytoken", "mysecrettoken");

        Mock::given(any())
            .respond_with(responder)
            .expect(1)
            .mount(&mock_server)
            .await;

        let mut client = Client::new(Config {
            avanza_username: String::from("user"),
            avanza_password: String::from("pass"),
            avanza_totp_secret: String::from("secret"),
        })
        .api_url(mock_server.uri());

        assert_ok!(
            client
                .borrow_mut()
                .authenticate_totp(String::from("4530ff65-a4d3-4af0-9e9b-22729a6157c9"))
                .await
        );

        assert_eq!("mysecrettoken", client.x_security_token);
        assert_eq!("4530ff65-a4d3-4af0-9e9b-22729a6157c9", client.session);
        assert_eq!(true, client.is_authenticated());
    }
}
