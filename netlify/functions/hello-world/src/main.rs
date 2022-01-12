use aws_lambda_events::event::apigw::{ApiGatewayProxyRequest, ApiGatewayProxyResponse};
use aws_lambda_events::encodings::Body;
use http::header::HeaderMap;
use lambda_runtime::{handler_fn, Context, Error};
use log::LevelFilter;
use simple_logger::SimpleLogger;
use std::env;

mod kraken;

#[tokio::main]
async fn main() -> Result<(), Error> {
    SimpleLogger::new().with_level(LevelFilter::Info).init().unwrap();

    let func = handler_fn(my_handler);
    lambda_runtime::run(func).await?;
    Ok(())
}

pub(crate) async fn my_handler(event: ApiGatewayProxyRequest, _ctx: Context) -> Result<ApiGatewayProxyResponse, Error> {
    let path = event.path.unwrap();

    let account = kraken::Account {
        key: String::from(option_env!("KRAKEN_API_KEY").unwrap()),
        secret: String::from(option_env!("KRAKEN_API_SECRET").unwrap()),
    };

    let balances = kraken::balance(&account)
        .await
        .expect("could not get balance");

    let resp = ApiGatewayProxyResponse {
        status_code: 200,
        headers: HeaderMap::new(),
        multi_value_headers: HeaderMap::new(),
        body: Some(Body::Text(format!("Balances: {:?}", balances))),
        is_base64_encoded: Some(false),
    };

    Ok(resp)
}