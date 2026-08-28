use std::{
    path::Path,
    process::{Child, Command, Stdio},
    time::Duration,
};

use hmac::{Hmac, Mac};
use serde_json::{json, Value};
use sha2::Sha256;

fn unused_port() -> u16 {
    let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
    listener.local_addr().unwrap().port()
}

fn start_with_only_port(directory: &Path, port: u16, production: bool) -> Child {
    let mut command = Command::new(env!("CARGO_BIN_EXE_webhook-quiet-hours"));
    command.env_clear().env("PORT", port.to_string());
    if production {
        // Mirrors the Dockerfile's baked-in, non-secret runtime mode.
        command.env("APP_ENV", "production");
    }
    command
        .current_dir(directory)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap()
}

async fn wait_until_ready(child: &mut Child, port: u16) -> reqwest::Client {
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{port}/health");
    for _ in 0..200 {
        assert!(
            child.try_wait().unwrap().is_none(),
            "server exited before becoming ready"
        );
        if let Ok(response) = client.get(&url).send().await {
            if response.status().is_success() {
                return client;
            }
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("server did not become ready at {url}");
}

fn stop_and_read_logs(mut child: Child) -> String {
    let _ = child.kill();
    let output = child.wait_with_output().unwrap();
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[tokio::test]
async fn production_binary_boots_with_only_port_and_reuses_persisted_secrets() {
    let directory = tempfile::tempdir().unwrap();
    let first_port = unused_port();
    let mut first = start_with_only_port(directory.path(), first_port, false);
    let client = wait_until_ready(&mut first, first_port).await;

    let admin_path = directory.path().join("data/admin-token");
    let key_path = directory.path().join("data/encryption-key");
    let admin_token = std::fs::read_to_string(&admin_path).unwrap();
    let encryption_key = std::fs::read_to_string(&key_path).unwrap();
    let admin_token = admin_token.trim();
    let encryption_key = encryption_key.trim();
    assert_eq!(admin_token.len(), 64);
    assert!(admin_token.bytes().all(|byte| byte.is_ascii_hexdigit()));
    assert_eq!(
        base64::Engine::decode(&base64::engine::general_purpose::STANDARD, encryption_key)
            .unwrap()
            .len(),
        32
    );

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&admin_path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(
            std::fs::metadata(&key_path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }

    let base = format!("http://127.0.0.1:{first_port}");
    let summary = client
        .get(format!("{base}/api/summary"))
        .bearer_auth(admin_token)
        .send()
        .await
        .unwrap();
    assert!(summary.status().is_success());
    let created: Value = client
        .post(format!("{base}/api/endpoints"))
        .bearer_auth(admin_token)
        .json(&json!({"name":"Restart proof","require_signature":true}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    let hook_url = reqwest::Url::parse(created["hook_url"].as_str().unwrap()).unwrap();
    let hook_path = hook_url.path().to_owned()
        + hook_url
            .query()
            .map(|query| format!("?{query}"))
            .as_deref()
            .unwrap_or("");
    let signing_secret = created["signing_secret"].as_str().unwrap().to_owned();

    let first_logs = stop_and_read_logs(first);
    assert!(first_logs.contains("\"admin_token_source\":\"generated\""));
    assert!(first_logs.contains("\"encryption_key_source\":\"generated\""));
    assert!(!first_logs.contains(admin_token));
    assert!(!first_logs.contains(encryption_key));

    let second_port = unused_port();
    let mut second = start_with_only_port(directory.path(), second_port, true);
    let client = wait_until_ready(&mut second, second_port).await;
    let body = br#"{"type":"invoice.failed","status":500,"id":"evt-restart"}"#;
    let mut mac = Hmac::<Sha256>::new_from_slice(signing_secret.as_bytes()).unwrap();
    mac.update(body);
    let signature = format!("sha256={}", hex::encode(mac.finalize().into_bytes()));
    let accepted = client
        .post(format!("http://127.0.0.1:{second_port}{hook_path}"))
        .header("x-webhook-signature", signature)
        .body(body.as_slice())
        .send()
        .await
        .unwrap();
    assert_eq!(accepted.status(), reqwest::StatusCode::ACCEPTED);

    assert_eq!(
        std::fs::read_to_string(&admin_path).unwrap().trim(),
        admin_token
    );
    assert_eq!(
        std::fs::read_to_string(&key_path).unwrap().trim(),
        encryption_key
    );
    let second_logs = stop_and_read_logs(second);
    assert!(second_logs.contains("\"admin_token_source\":\"persisted\""));
    assert!(second_logs.contains("\"encryption_key_source\":\"persisted\""));
}
