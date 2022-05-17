pub struct ShadowAuthenticator;

impl ShadowAuthenticator {
    pub fn new() -> Self {
        Self
    }
}

impl super::Authenticator for ShadowAuthenticator {
    fn auth(&self, username: &str, password: &str) -> anyhow::Result<bool> {
        let hash = if let Some(hash) = shadow::Shadow::from_name(username) {
            hash
        } else {
            return Ok(false)
        };

        Ok(pwhash::unix::verify(password, &hash.password))
    }
}
