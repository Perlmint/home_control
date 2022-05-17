pub struct PamAuthenticator {
    service_name: String,
}

impl PamAuthenticator {
    pub fn new(service_name: &str) -> Self {
        PamAuthenticator {
            service_name: service_name.to_string(),
        }
    }
}

impl super::Authenticator for PamAuthenticator {
    fn auth(&self, username: &str, password: &str) -> anyhow::Result<bool> {
        let mut authenticator = pam::Authenticator::with_password(&self.service_name)?;
        authenticator
            .get_handler()
            .set_credentials(username, password);

        Ok(authenticator.authenticate().is_ok())
    }
}
