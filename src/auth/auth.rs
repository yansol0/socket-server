use serde::{Serialize, Deserialize};
use jsonwebtoken::{decode, DecodingKey, TokenData, Validation, Algorithm};
use jsonwebtoken::errors::Error;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub id: String,
    iat: usize,
    exp: usize,
}

pub fn verify_token(token: &str, secret: &str) -> Result<TokenData<Claims>, Error> {
    return decode::<Claims>(token, &DecodingKey::from_secret(secret.as_ref()), &Validation::new(Algorithm::HS256));
}
