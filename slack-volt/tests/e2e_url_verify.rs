mod common;

#[tokio::test]
async fn test_url_verification_challenge() {
    let app = common::test_app_builder("http://unused").build();

    let body = r#"{"type":"url_verification","challenge":"test_challenge_xyz"}"#;
    let headers = common::sign_json_request(body);
    let result = app
        .dispatch_async("application/json", body, headers)
        .await
        .unwrap();

    assert_eq!(result.text.as_deref(), Some("test_challenge_xyz"));
}

#[tokio::test]
async fn test_url_verification_with_different_challenge() {
    let app = common::test_app_builder("http://unused").build();

    let body = r#"{"type":"url_verification","challenge":"abc123_random_challenge"}"#;
    let headers = common::sign_json_request(body);
    let result = app
        .dispatch_async("application/json", body, headers)
        .await
        .unwrap();

    assert_eq!(result.text.as_deref(), Some("abc123_random_challenge"));
}
