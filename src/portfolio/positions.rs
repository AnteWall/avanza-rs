use crate::client::Client;
use crate::error::RequestError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PositionsResponse {
    instrument_positions: Vec<InstrumentPositions>,
    total_profit: f64,
    total_profit_percent: f64,
    total_balance: f64,
    total_own_capital: f64,
    total_buying_power: f64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstrumentPositions {
    instrument_type: String,
    positions: Vec<Positions>,
    todays_profit_percent: f64,
    total_profit_percent: f64,
    total_profit_value: f64,
    total_value: f64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Positions {
    account_id: String,
    account_name: String,
    account_type: String,
    acquired_value: f64,
    average_acquired_price: f64,
    change: f64,
    change_percent: f64,
    currency: String,
    depositable: bool,
    flag_code: String,
    last_price: f64,
    last_price_updated: String,
    name: String,
    orderbook_id: String,
    profit: f64,
    profit_percent: f64,
    tradable: bool,
    value: f64,
    volume: i64,
}

impl Client {
    async fn get_positions(mut self) -> Result<PositionsResponse, RequestError> {
        if !self.is_authenticated() {
            return Err(RequestError::NotAuthenticatedError());
        }
        let uri = format!("{}/_mobile/account/positions", self.api_url);
        let resp = self.get_response::<PositionsResponse>(&uri).await?;
        Ok(resp)
    }
}

#[cfg(test)]
mod tests {
    use crate::client::Config;

    use super::*;
    use tokio_test::{assert_err, assert_ok};
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn mock_auth(mock_server: &MockServer) {
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
    }

    #[tokio::test]
    async fn require_auth() {
        let client = Client::new(Config {
            avanza_username: String::from("user"),
            avanza_password: String::from("pass"),
            avanza_totp_secret: String::from("secret"),
        })
        .api_url(String::from("http://fake-url.com"));
        assert_err!(client.get_positions().await, "unauthorized");
    }

    #[tokio::test]
    async fn can_get_positions() {
        let mock_server = MockServer::start().await;

        let responder = ResponseTemplate::new(200).set_body_string(
            String::from("{\"instrumentPositions\":[],\"totalOwnCapital\":100000,\"totalProfit\":40000,\"totalBuyingPower\":4000,\"totalBalance\":4000,\"totalProfitPercent\":10}")
        );

        mock_auth(&mock_server).await;

        Mock::given(method("GET"))
            .and(path("/_mobile/account/positions"))
            .respond_with(responder)
            .mount(&mock_server)
            .await;

        let mut client = Client::new(Config {
            avanza_username: String::from("user"),
            avanza_password: String::from("pass"),
            avanza_totp_secret: String::from("secret"),
        })
        .api_url(mock_server.uri());

        client.authenticate().await.expect("failed to authenticate");

        let positions = assert_ok!(client.get_positions().await);

        assert_eq!(positions.total_balance, 4000.0)
    }
}
