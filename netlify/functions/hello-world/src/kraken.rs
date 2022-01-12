use std::collections::HashMap;
use serde::Deserialize;
use reqwest;
use crypto::digest::Digest;
use crypto::hmac::Hmac;
use crypto::mac::Mac;
use crypto::sha2::{Sha256, Sha512};

#[derive(Deserialize, Debug)]
pub struct KrakenResponse<T> {
	result: Option<T>,
	error: Vec<String>,
}
pub struct Account {
	pub key: String,
	pub secret: String,
}

async fn private(account: &Account, method: &str, params: &mut HashMap<String, String>) -> Result<KrakenResponse<HashMap<String, String>>, reqwest::Error> {
	let path = format!("/0/private/{}", method);
    let url = format!("https://api.kraken.com{}", path);
	let timestamp = ::std::time::UNIX_EPOCH.elapsed().unwrap();
    let nonce = format!("{}{:09}", timestamp.as_secs(), timestamp.subsec_nanos());

	params.insert("nonce".to_owned(), nonce.clone());

	let mut body = params.iter().fold(
        String::new(),
        |data, item| data + item.0 + "=" + item.1 + "&",
    );
    body.pop();
	
	let body_bytes = body.as_bytes();
    let secret = base64::decode(&account.secret).unwrap();
    let mut hmac = Hmac::new(Sha512::new(), &secret);
    let mut body_hasher = Sha256::new();

    body_hasher.input(nonce.as_bytes());
    body_hasher.input(body_bytes);

    hmac.input(path.as_bytes());
    let mut out: [u8; 32] = [0; 32];
    body_hasher.result(&mut out);
    hmac.input(&out);

    let sign = base64::encode(hmac.result().code());

	let http_response = reqwest::Client::new()
		.post(url)
		.header("API-Key", &account.key)
		.header("API-Sign", sign)
		.form(params)
		.send()
		.await?;

	let response = http_response
        .json::<KrakenResponse<HashMap<String, String>>>()
        .await?;
	
	Ok(response)
}

pub async fn balance(account: &Account) -> Result<HashMap<String, String>, String> {
    let mut params = HashMap::new();
    private(account, "Balance", &mut params)
		.await
		.map_err(|e| format!("{:?}", e))
		.and_then(
			|response| if response.error.len() > 0 {
				Err(format!("{:?}", response.error))
			} else {
				Ok(response.result.unwrap())
			}
		)
}
