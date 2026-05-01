//! Supabase JWT validation using JWKS.

use anyhow::{Context, Result};
use base64::Engine;
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header};
use serde::Deserialize;

/// Extracted claims used by the game server.
#[derive(Debug, Clone)]
pub struct Claims {
    pub sub: String,
    pub email: Option<String>,
    pub preferred_username: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawClaims {
    sub: String,
    #[allow(dead_code)]
    exp: usize,
    #[serde(default)]
    email: Option<String>,
    #[serde(default)]
    preferred_username: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Jwks {
    keys: Vec<Jwk>,
}

#[derive(Debug, Deserialize)]
struct Jwk {
    kid: Option<String>,
    kty: String,
    n: Option<String>,
    e: Option<String>,
    x5c: Option<Vec<String>>,
}

/// Validate a Supabase JWT against JWKS and expected issuer/audience.
pub async fn validate_supabase_jwt(
    token: &str,
    jwks_url: &str,
    expected_issuer: Option<&str>,
    expected_audience: Option<&str>,
) -> Result<Claims> {
    let header = decode_header(token).context("failed to decode JWT header")?;
    let algorithm = header.alg;
    let kid = header.kid.clone().context("JWT missing kid header")?;

    let jwks = reqwest::Client::new()
        .get(jwks_url)
        .send()
        .await
        .context("failed to fetch JWKS")?
        .error_for_status()
        .context("JWKS endpoint returned non-success status")?
        .json::<Jwks>()
        .await
        .context("failed to parse JWKS json")?;

    let key = jwks
        .keys
        .into_iter()
        .find(|k| k.kid.as_deref() == Some(kid.as_str()))
        .context("matching JWKS key id not found")?;

    let decoding_key = if key.kty == "RSA" && algorithm == Algorithm::RS256 {
        let n = key.n.context("JWKS RSA key missing modulus n")?;
        let e = key.e.context("JWKS RSA key missing exponent e")?;
        DecodingKey::from_rsa_components(&n, &e).context("invalid RSA components in JWKS")?
    } else if key.kty == "EC" && algorithm == Algorithm::ES256 {
        let x5c = key.x5c.context("JWKS EC key missing x5c cert chain")?;
        let cert_b64 = x5c.first().context("JWKS x5c cert chain empty")?;
        let cert_der = base64::engine::general_purpose::STANDARD
            .decode(cert_b64)
            .context("invalid base64 x5c certificate")?;
        DecodingKey::from_ec_der(&cert_der)
    } else {
        anyhow::bail!(
            "unsupported JWKS key type/algorithm combination: {}/ {:?}",
            key.kty,
            algorithm
        )
    };

    let mut validation = Validation::new(algorithm);
    validation.set_required_spec_claims(&["exp", "sub"]);
    if let Some(issuer) = expected_issuer {
        validation.set_issuer(&[issuer]);
    }
    if let Some(aud) = expected_audience {
        validation.set_audience(&[aud]);
    }

    let decoded = decode::<RawClaims>(token, &decoding_key, &validation)
        .context("JWT signature or claims validation failed")?;

    Ok(Claims {
        sub: decoded.claims.sub,
        email: decoded.claims.email,
        preferred_username: decoded.claims.preferred_username,
    })
}
