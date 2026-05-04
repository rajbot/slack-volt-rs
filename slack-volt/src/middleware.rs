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
        let basestring = format!("v0:{timestamp}:{body}");
        let mut mac =
            Hmac::<Sha256>::new_from_slice(self.signing_secret.as_bytes()).expect("valid key");
        mac.update(basestring.as_bytes());
        let result = mac.finalize();
        let computed = format!("v0={}", hex::encode(result.into_bytes()));
        computed == expected_signature
    }
}

impl Middleware for SignatureVerifier {
    fn process(&self, headers: &Headers, body: &str) -> Result<(), crate::Error> {
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
        let headers = Headers {
            timestamp: "123".to_string(),
            signature: "v0=bad".to_string(),
            content_type: "application/json".to_string(),
        };
        let result = verifier.process(&headers, "body");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            crate::Error::SignatureVerification(_)
        ));
    }
}
