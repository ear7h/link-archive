use std::convert::TryInto;
use std::time;

use jsonwebtoken as jwt;
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};

use crate::{Error, Result};

pub fn encode_password(pass : &[u8]) -> Result<String> {
    let mut salt = [0; 32];

    tokio::task::block_in_place(|| {
        thread_rng().fill(&mut salt);
    });

    Ok(argon2::hash_encoded(pass, &salt, &Default::default())?)
}

pub fn verify_password(encoded : &str, pass : &[u8]) -> Result<bool> {
    Ok(argon2::verify_encoded(encoded, pass)?)
}

pub struct Token {
    pub iss :     String,
    pub aud :     String,
    pub sub :     String,
    pub version : u32,
}

impl Token {
    pub fn issue(
        &self,
        secret : &[u8],
        exp_duration : time::Duration,
    ) -> Result<String> {
        let now = time::SystemTime::now();

        let iat = now
            .duration_since(time::UNIX_EPOCH)?
            .as_secs()
            .try_into()
            .unwrap();

        let exp = now
            .checked_add(exp_duration)
            .ok_or(Error::TokenDurationTooBig)?
            .duration_since(time::UNIX_EPOCH)?
            .as_secs()
            .try_into()
            .unwrap();

        #[derive(Serialize)]
        pub struct TokenFull<'a> {
            iss :     &'a str,
            aud :     &'a str,
            sub :     &'a str,
            version : u32,
            iat :     u64,
            exp :     u64,
        }

        let tok = TokenFull {
            iss : &self.iss,
            aud : &self.aud,
            sub : &self.sub,
            version : self.version,
            iat,
            exp,
        };

        Ok(jwt::encode(
            &Default::default(),
            &tok,
            &jwt::EncodingKey::from_secret(secret),
        )
        .map_err(|err| err.into_kind())?)
    }

    pub fn validate(token : &str, secret : &[u8], iss : &str) -> Result<Self> {
        let validation = jwt::Validation {
            validate_exp : true,
            iss : Some(iss.to_string()),
            ..Default::default()
        };

        #[derive(Deserialize)]
        #[allow(dead_code)]
        pub struct TokenFull {
            iss :     String,
            aud :     String,
            sub :     String,
            version : u32,
            iat :     u64,
            exp :     u64,
        }

        let tok : TokenFull = jwt::decode(
            token,
            &jwt::DecodingKey::from_secret(secret),
            &validation,
        )
        .map_err(|err| err.into_kind())?
        .claims;

        Ok(Self {
            iss :     tok.iss,
            aud :     tok.aud,
            sub :     tok.sub,
            version : tok.version,
        })
    }
}
