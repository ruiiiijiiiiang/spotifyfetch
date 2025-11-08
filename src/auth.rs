use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    error::Error,
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
use url::Url;

const CLIENT_ID: &str = "ebdbdb22841c48648acf563e594d928e";
const TOKEN_URL: &str = "https://accounts.spotify.com/api/token";
const AUTHORIZE_URL: &str = "https://accounts.spotify.com/authorize";
const REDIRECT_URI: &str = "http://localhost:8888/callback";
const LOCALHOST: &str = "127.0.0.1";
const PORT: u16 = 8888;
const AUTH_SCOPE: [&str; 1] = ["user-top-read"];

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AuthToken {
    access_token: String,
    refresh_token: String,
    expires_at: u64,
}

impl AuthToken {
    pub async fn get_valid_token() -> Result<String, Box<dyn Error>> {
        match Self::load() {
            Ok(mut token_data) => {
                if token_data.is_expired() {
                    println!("Access token expired, refreshing...");
                    token_data = Self::refresh_access_token(&token_data.refresh_token).await?;
                    token_data.save()?;
                    println!("Token refreshed successfully!");
                }
                Ok(token_data.access_token)
            }
            Err(_) => {
                println!("No tokens found, starting authorization flow...");
                let auth = Auth::new();
                let token_data = auth.perform_oauth().await?;
                token_data.save()?;
                Ok(token_data.access_token)
            }
        }
    }

    fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        now >= self.expires_at - 60
    }

    fn save(&self) -> Result<(), Box<dyn Error>> {
        let path = Self::get_token_path();
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }

    fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let path = Self::get_token_path();
        let json = fs::read_to_string(path)?;
        let token_data = serde_json::from_str(&json)?;
        Ok(token_data)
    }

    fn get_token_path() -> PathBuf {
        let mut path = dirs::config_dir().expect("Could not find config directory");
        path.push("spotifyfetch");
        fs::create_dir_all(&path).ok();
        path.push("tokens.json");
        path
    }

    async fn refresh_access_token(refresh_token: &str) -> Result<Self, Box<dyn Error>> {
        let client = reqwest::Client::new();

        let params = [
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", CLIENT_ID),
        ];

        let response = client.post(TOKEN_URL).form(&params).send().await?;

        #[derive(Deserialize)]
        struct RefreshResponse {
            access_token: String,
            refresh_token: Option<String>, // Sometimes Spotify returns a new one
            expires_in: u64,
        }

        let refresh_response: RefreshResponse = response.json().await?;

        let expires_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + refresh_response.expires_in;

        Ok(AuthToken {
            access_token: refresh_response.access_token,
            refresh_token: refresh_response
                .refresh_token
                .unwrap_or_else(|| refresh_token.to_string()),
            expires_at,
        })
    }
}

struct Auth {
    code_verifier: String,
    auth_url: String,
}

#[derive(Deserialize, Debug)]
struct TokenResponse {
    access_token: String,
    refresh_token: String,
    expires_in: u64,
}

impl Auth {
    fn new() -> Self {
        let code_verifier = Self::generate_code_verifier();
        let code_challenge = Self::generate_code_challenge(&code_verifier);
        let auth_url = Self::build_auth_url(&code_challenge);

        Auth {
            code_verifier,
            auth_url,
        }
    }

    async fn perform_oauth(&self) -> Result<AuthToken, Box<dyn Error>> {
        println!("Opening browser for authorization...");
        open::that(self.auth_url.clone())?;

        let code = Self::wait_for_callback()?;

        let token_response = self.exchange_code_for_token(&code).await?;

        let expires_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + token_response.expires_in;

        Ok(AuthToken {
            access_token: token_response.access_token,
            refresh_token: token_response.refresh_token,
            expires_at,
        })
    }

    fn wait_for_callback() -> Result<String, Box<dyn Error>> {
        let server = tiny_http::Server::http(format!("{}:{}", LOCALHOST, PORT)).unwrap();
        println!("Waiting for authorization callback...");

        let request = server.recv()?;
        let url = format!("http://{}{}", LOCALHOST, request.url());
        let parsed_url = Url::parse(&url)?;

        let code = parsed_url
            .query_pairs()
            .find(|(key, _)| key == "code")
            .map(|(_, value)| value.to_string())
            .ok_or("No code found in callback")?;

        let response = tiny_http::Response::from_string(
            "Authorization successful! You can close this window.",
        );
        request.respond(response)?;

        Ok(code)
    }

    async fn exchange_code_for_token(&self, code: &str) -> Result<TokenResponse, Box<dyn Error>> {
        let client = reqwest::Client::new();

        let params = [
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", REDIRECT_URI),
            ("client_id", CLIENT_ID),
            ("code_verifier", &self.code_verifier),
        ];

        let response = client
            .post(TOKEN_URL)
            .form(&params)
            .send()
            .await?
            .json::<TokenResponse>()
            .await?;

        Ok(response)
    }

    fn generate_code_verifier() -> String {
        let random_bytes: Vec<u8> = (0..32).map(|_| rand::rng().random::<u8>()).collect();
        URL_SAFE_NO_PAD.encode(random_bytes)
    }

    fn generate_code_challenge(verifier: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(verifier.as_bytes());
        let result = hasher.finalize();
        URL_SAFE_NO_PAD.encode(result)
    }

    fn build_auth_url(code_challenge: &str) -> String {
        let mut url = Url::parse(AUTHORIZE_URL).unwrap();
        url.query_pairs_mut()
            .append_pair("client_id", CLIENT_ID)
            .append_pair("response_type", "code")
            .append_pair("redirect_uri", REDIRECT_URI)
            .append_pair("code_challenge_method", "S256")
            .append_pair("code_challenge", code_challenge)
            .append_pair("scope", &AUTH_SCOPE.join(" "));

        url.to_string()
    }
}

impl Default for Auth {
    fn default() -> Self {
        Self::new()
    }
}
