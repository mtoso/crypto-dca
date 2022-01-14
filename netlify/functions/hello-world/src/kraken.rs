use core::fmt;
use std::collections::HashMap;
use serde::Deserialize;
use reqwest;
use crypto::digest::Digest;
use crypto::hmac::Hmac;
use crypto::mac::Mac;
use crypto::sha2::{Sha256, Sha512};
use serde::de::DeserializeOwned;

#[derive(Debug, Clone, Copy)]
pub enum OrderType {
    Market,
    /// (price = limit price)
    Limit,
    /// (price = stop loss price)
    StopLoss,
    /// (price = take profit price)
    TakeProfit,
    /// (price = stop loss trigger price, price2 = triggered limit price)
    StopLossLimit,
    /// (price = take profit trigger price, price2 = triggered limit price)
    TakeProfitLimit,
    SettlePosition,
}
impl fmt::Display for OrderType {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			OrderType::Market => write!(f, "market"),
			OrderType::Limit => write!(f, "limit"),
			OrderType::StopLoss => write!(f, "stop-loss"),
			OrderType::TakeProfit => write!(f, "take-profit"),
			OrderType::StopLossLimit => write!(f, "stop-loss-limit"),
			OrderType::TakeProfitLimit => write!(f, "take-profit-limit"),
			OrderType::SettlePosition => write!(f, "settle-position")
		}
	}
}

#[derive(Debug, Clone, Copy)]
pub enum OrderDirection {
    Buy,
    Sell,
}

impl fmt::Display for OrderDirection {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			OrderDirection::Buy => write!(f, "buy"),
			OrderDirection::Sell => write!(f, "sell")
		}
	}
}

pub struct NewOrder { 
    /// asset pair
    pub pair: String,
    /// order direction (buy/sell)
    pub order_direction: OrderDirection,
    pub order_type: OrderType,
    /// price (optional.  dependent upon ordertype)
    pub price: Option<String>,
    /// secondary price (optional.  dependent upon ordertype)
    pub price2: Option<String>,
    /// order volume in lots
    pub volume: Option<String>,
    /// amount of leverage desired (optional.  default = none)
    pub leverage: Option<String>,
    /// comma delimited list of order flags (optional):
    ///   + viqc = volume in quote currency (not available for leveraged orders)
    ///   + fcib = prefer fee in base currency
    ///   + fciq = prefer fee in quote currency
    ///   + nompp = no market price protection
    ///   + post = post only order (available when ordertype = limit)
    pub oflags: Option<String>,
    /// scheduled start time (optional):
    ///   + 0 = now (default)
    ///   + +<n> = schedule start time <n> seconds from now
    ///   + <n> = unix timestamp of start time
    pub starttm: Option<i64>,
    /// expiration time (optional):
    ///   + 0 = no expiration (default)
    ///   + +<n> = expire <n> seconds from now
    ///   + <n> = unix timestamp of expiration time
    pub expiretm: Option<i64>,
    /// user reference id.  32-bit signed number.  (optional)
    pub userref: Option<String>,
    /// validate inputs only.  do not submit order (optional)
    pub validate: Option<bool>,
}

#[derive(Deserialize, Debug)]
pub struct ApiResponse<T> {
	result: Option<T>,
	error: Vec<String>,
}
pub struct Account {
	pub key: String,
	pub secret: String,
}

#[derive(Deserialize, Debug)]
pub struct AddOrderResponse {
	descr: HashMap<String, String>,
	txid: Option<Vec<String>>
}

async fn private<T>(account: &Account, method: &str, params: &mut HashMap<String, String>) -> Result<ApiResponse<T>, reqwest::Error> 
	where
		T: DeserializeOwned
{
	let path = format!("/0/private/{}", method);
    let url = format!("https://api.kraken.com{}", path);
	let timestamp = ::std::time::UNIX_EPOCH.elapsed().unwrap();
    let nonce = format!("{}{:09}", timestamp.as_secs(), timestamp.subsec_nanos());

	params.insert("nonce".to_owned(), nonce.clone());

	let mut body = params.iter().fold(
        String::new(),
        |data, item| data + item.0 + "=" + item.1 + "&",
    );
    body.pop(); // remove last &
	
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
		.await?
		.error_for_status()?;
	
	let response = http_response
        .json::<ApiResponse<T>>()
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

pub async fn add_order(account: &Account, order: NewOrder) -> Result<AddOrderResponse, String> {
	let mut params = HashMap::new();

	params.insert("pair".to_owned(), order.pair.to_string());
	params.insert("type".to_owned(), order.order_direction.to_string());
	params.insert("ordertype".to_owned(), order.order_type.to_string());

	if let Some(price) = order.price {
		params.insert("price".to_owned(), price);
	}

	if let Some(price) = order.price2 {
        params.insert("price2".to_owned(), price);
    }

	if let Some(volume) = order.volume {
        params.insert("volume".to_owned(), volume);
    }	

    if let Some(leverage) = order.leverage {
        params.insert("leverage".to_owned(), leverage);
    }

    if let Some(oflags) = order.oflags {
        params.insert("oflags".to_owned(), oflags);
    }

    if let Some(userref) = order.userref {
        params.insert("userref".to_owned(), userref);
    }

    if let Some(starttm) = order.starttm {
        params.insert("starttm".to_owned(), format!("{}", starttm));
    }

    if let Some(expiretm) = order.expiretm {
        params.insert("expiretm".to_owned(), format!("{}", expiretm));
    }

    if order.validate.is_some() {
        params.insert("validate".to_owned(), String::from("1"));
    }

	private(account, "AddOrder", &mut params)
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