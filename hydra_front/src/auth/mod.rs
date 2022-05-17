#[cfg(feature = "auth-pam")]
pub mod pam;
#[cfg(feature = "auth-shadow")]
pub mod shadow;

pub trait Authenticator {
    fn auth(&self, username: &str, password: &str) -> anyhow::Result<bool>;
}