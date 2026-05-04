use hmac::{Hmac, Mac};
use sha2::Sha256;

pub trait Middleware: Send + Sync + 'static {
    fn process(&self, headers: &Headers, body: &str) -> Result<(), crate::Error>;
}

pub struct Headers {
    pub timestamp: String,
    pub signature: String,
    pub content_type: String,
}

pub struct SignatureVerifier {
    signing_secret: String,
}

impl SignatureVerifier {
    pub fn new(signing_secret: String) -> Self {
        SignatureVerifier { signing_secret }
    }

    pub fn verify(&self, timestamp: &str, body: &str, expected_signature: &str) -> bool {
        let Some(hex_sig) = expected_signature.strip_prefix("v0=") else {
            return false;
        };
        let Ok(expected_bytes) = hex::decode(hex_sig) else {
            return false;
        };

        let basestring = format!("v0:{timestamp}:{body}");
        let mut mac =
            Hmac::<Sha256>::new_from_slice(self.signing_secret.as_bytes()).expect("valid key");
        mac.update(basestring.as_bytes());
        mac.verify_slice(&expected_bytes).is_ok()
    }
}

const MAX_TIMESTAMP_AGE_SECS: i64 = 300;

impl Middleware for SignatureVerifier {
    fn process(&self, headers: &Headers, body: &str) -> Result<(), crate::Error> {
        let ts: i64 = headers.timestamp.parse().map_err(|_| {
            crate::Error::SignatureVerification("invalid timestamp".to_string())
        })?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock before epoch")
            .as_secs() as i64;
        if (now - ts).abs() > MAX_TIMESTAMP_AGE_SECS {
            return Err(crate::Error::SignatureVerification(
                "request timestamp too old".to_string(),
            ));
        }

        if !self.verify(&headers.timestamp, body, &headers.signature) {
            return Err(crate::Error::SignatureVerification(
                "invalid request signature".to_string(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn compute_signature(secret: &str, timestamp: &str, body: &str) -> String {
        use hmac::Mac;
        let basestring = format!("v0:{timestamp}:{body}");
        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(basestring.as_bytes());
        let result = mac.finalize();
        format!("v0={}", hex::encode(result.into_bytes()))
    }

    #[test]
    fn test_signature_verify_valid() {
        let secret = "test_secret_123";
        let timestamp = "1625000000";
        let body = "command=%2Fhello&text=world";
        let sig = compute_signature(secret, timestamp, body);

        let verifier = SignatureVerifier::new(secret.to_string());
        assert!(verifier.verify(timestamp, body, &sig));
    }

    #[test]
    fn test_signature_verify_invalid() {
        let verifier = SignatureVerifier::new("secret".to_string());
        assert!(!verifier.verify("123", "body", "v0=deadbeef"));
    }

    #[test]
    fn test_signature_verify_wrong_secret() {
        let timestamp = "1625000000";
        let body = "hello";
        let sig = compute_signature("secret_a", timestamp, body);

        let verifier = SignatureVerifier::new("secret_b".to_string());
        assert!(!verifier.verify(timestamp, body, &sig));
    }

    #[test]
    fn test_middleware_rejects_bad_sig() {
        let verifier = SignatureVerifier::new("secret".to_string());
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .to_string();
        let headers = Headers {
            timestamp: now,
            signature: "v0=deadbeef".to_string(),
            content_type: "application/json".to_string(),
        };
        let result = verifier.process(&headers, "body");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            crate::Error::SignatureVerification(_)
        ));
    }

    #[test]
    fn test_middleware_rejects_stale_timestamp() {
        let verifier = SignatureVerifier::new("secret".to_string());
        let headers = Headers {
            timestamp: "1000000000".to_string(),
            signature: "v0=abc".to_string(),
            content_type: "application/json".to_string(),
        };
        let result = verifier.process(&headers, "body");
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("too old"));
    }

    #[test]
    fn test_middleware_rejects_invalid_timestamp() {
        let verifier = SignatureVerifier::new("secret".to_string());
        let headers = Headers {
            timestamp: "not_a_number".to_string(),
            signature: "v0=abc".to_string(),
            content_type: "application/json".to_string(),
        };
        let result = verifier.process(&headers, "body");
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("invalid timestamp"));
    }

    #[test]
    fn test_middleware_accepts_valid_request() {
        let secret = "test_signing_secret";
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .to_string();
        let body = "command=%2Fhello&text=world";
        let sig = compute_signature(secret, &now, body);

        let verifier = SignatureVerifier::new(secret.to_string());
        let headers = Headers {
            timestamp: now,
            signature: sig,
            content_type: "application/x-www-form-urlencoded".to_string(),
        };
        let result = verifier.process(&headers, body);
        assert!(result.is_ok());
    }

    #[test]
    fn test_signature_missing_v0_prefix() {
        let verifier = SignatureVerifier::new("secret".to_string());
        assert!(!verifier.verify("123", "body", "no_prefix_here"));
    }

    #[test]
    fn test_signature_invalid_hex() {
        let verifier = SignatureVerifier::new("secret".to_string());
        assert!(!verifier.verify("123", "body", "v0=not_valid_hex_zzz"));
    }

    #[test]
    fn test_middleware_rejects_empty_timestamp() {
        let verifier = SignatureVerifier::new("secret".to_string());
        let headers = Headers {
            timestamp: String::new(),
            signature: "v0=abc".to_string(),
            content_type: "application/json".to_string(),
        };
        let result = verifier.process(&headers, "body");
        assert!(result.is_err());
    }

    #[test]
    fn test_middleware_rejects_empty_signature() {
        let secret = "secret";
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .to_string();
        let verifier = SignatureVerifier::new(secret.to_string());
        let headers = Headers {
            timestamp: now,
            signature: String::new(),
            content_type: "application/json".to_string(),
        };
        let result = verifier.process(&headers, "body");
        assert!(result.is_err());
    }

    #[test]
    fn test_timestamp_boundary_just_within_window() {
        let secret = "secret";
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let ts = (now - 299).to_string();
        let body = "test";
        let sig = compute_signature(secret, &ts, body);

        let verifier = SignatureVerifier::new(secret.to_string());
        let headers = Headers {
            timestamp: ts,
            signature: sig,
            content_type: "text/plain".to_string(),
        };
        assert!(verifier.process(&headers, body).is_ok());
    }

    #[test]
    fn test_timestamp_boundary_just_outside_window() {
        let secret = "secret";
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let ts = (now - 301).to_string();
        let body = "test";
        let sig = compute_signature(secret, &ts, body);

        let verifier = SignatureVerifier::new(secret.to_string());
        let headers = Headers {
            timestamp: ts,
            signature: sig,
            content_type: "text/plain".to_string(),
        };
        let result = verifier.process(&headers, body);
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_with_empty_body() {
        let secret = "secret";
        let ts = "1625000000";
        let sig = compute_signature(secret, ts, "");

        let verifier = SignatureVerifier::new(secret.to_string());
        assert!(verifier.verify(ts, "", &sig));
    }
}
