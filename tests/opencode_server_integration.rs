use oya_lite::lifecycle::types::{
    OpencodeServerConfig, OpencodeUrl, Username, SensitiveString,
};

#[test]
fn opencode_server_config_display() {
    let config = OpencodeServerConfig {
        url: OpencodeUrl("http://localhost:4099".into()),
        username: Username("user".into()),
        password: SensitiveString("secret".into()),
    };
    let s = format!("{:?}", config);
    assert!(s.contains("localhost:4099"));
    assert!(!s.contains("secret"));
}

#[test]
fn opencode_server_config_url_accessors() {
    let config = OpencodeServerConfig {
        url: OpencodeUrl("http://localhost:4099".into()),
        username: Username("user".into()),
        password: SensitiveString("secret".into()),
    };
    assert_eq!(config.url.as_str(), "http://localhost:4099");
}

#[test]
fn opencode_server_config_username_accessors() {
    let config = OpencodeServerConfig {
        url: OpencodeUrl("http://localhost:4099".into()),
        username: Username("admin".into()),
        password: SensitiveString("hunter2".into()),
    };
    assert_eq!(config.username.as_str(), "admin");
}

#[test]
fn opencode_server_config_password_redacted() {
    let config = OpencodeServerConfig {
        url: OpencodeUrl("http://localhost:4099".into()),
        username: Username("admin".into()),
        password: SensitiveString("hunter2".into()),
    };
    assert_eq!(config.password.as_str(), "hunter2");
    let debug = format!("{:?}", config);
    assert!(!debug.contains("hunter2"));
    assert!(debug.contains("***"));
}

#[test]
fn opencode_server_config_clone() {
    let config = OpencodeServerConfig {
        url: OpencodeUrl("http://localhost:4099".into()),
        username: Username("user".into()),
        password: SensitiveString("secret".into()),
    };
    let cloned = config.clone();
    assert_eq!(cloned.url, config.url);
    assert_eq!(cloned.username, config.username);
    assert_eq!(cloned.password.as_str(), config.password.as_str());
}

#[test]
fn opencode_url_display() {
    let url = OpencodeUrl("http://localhost:4099".into());
    assert_eq!(format!("{}", url), "http://localhost:4099");
}

#[test]
fn opencode_url_https() {
    let url = OpencodeUrl("https://opencode.example.com".into());
    assert_eq!(format!("{}", url), "https://opencode.example.com");
}

#[test]
fn sensitive_string_debug_shows_redaction() {
    let s = SensitiveString("my-super-secret".into());
    let debug = format!("{:?}", s);
    assert!(debug.contains("***"));
    assert!(!debug.contains("my-super-secret"));
}

#[test]
fn sensitive_string_as_str_works() {
    let s = SensitiveString("actual-value".into());
    assert_eq!(s.as_str(), "actual-value");
}

#[test]
fn username_display() {
    let u = Username("testuser".into());
    assert_eq!(format!("{}", u), "testuser");
}

#[test]
fn opencode_url_with_port() {
    let url = OpencodeUrl("http://127.0.0.1:8080".into());
    assert!(url.as_str().contains("8080"));
}

#[test]
fn opencode_server_config_with_path() {
    let config = OpencodeServerConfig {
        url: OpencodeUrl("http://localhost:4099/api/v1".into()),
        username: Username("user".into()),
        password: SensitiveString("pass".into()),
    };
    assert!(config.url.as_str().contains("/api/v1"));
}
