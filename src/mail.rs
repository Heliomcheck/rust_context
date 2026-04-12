use lettre::{
    SmtpTransport, Transport, message::{Mailbox, MessageBuilder},
    transport::smtp::authentication::Credentials,};
use anyhow::Result;
use dotenvy::dotenv;
use std::{env, hash::Hash};
use anyhow::Context;
use lettre::transport::smtp::client::Tls;
use std::collections::HashMap;
use std::sync::Arc;


pub struct VerificationCode {
    pub codes: HashMap<String, u32>, // email -> code
    pub created_at: HashMap<String, chrono::DateTime<chrono::Utc>>, // email -> created_at
    pub expires_at: HashMap<String, chrono::DateTime<chrono::Utc>> // email -> expires_at
}

impl VerificationCode {
    pub fn new() -> Self {
        Self {
            codes: HashMap::new(),
            created_at: HashMap::new(),
            expires_at: HashMap::new()
        }
    }
}

pub async fn send_mail_verif_code(to_mail: &str, state: Arc<crate::AppState>) -> Result<(), anyhow::Error>{
    dotenv().ok();

    let username = env::var("SMTP_USERNAME").context("Username is not valid")?;
    let password = env::var("SMTP_PASSWORD").context("Password is not valid")?;
    let server = env::var("SMTP_SERVER").context("Server is not valid")?;
    let port = env::var("SMTP_PORT") 
        .context("Port is not valid")?
        .parse::<u16>()
        .context("Port must be a number")?;

    let username_format = username.parse::<Mailbox>().context("Error 'from' (mail)")?;
    let to_mail_format = to_mail.parse::<Mailbox>().context("Error 'to_mail' (mail)")?;

    let code = crate::generator::Generator::verification_code();
    let email = MessageBuilder::new()
        .from(username_format)
        .to(to_mail_format)
        .subject("Your verification code")
        .body(format!("Verification code {}", code)).context("Code not found")?;

    let creds = Credentials::new(username.to_owned(), password.to_owned());

    let mailer = SmtpTransport::relay(&server)?
        .credentials(creds)
        .port(port)
        .tls(Tls::None)
        .build();

    let code_u32 = code.parse::<u32>().context("Code must be a number")?;
    
    match mailer.send(&email) {
        Ok(_) => {
            state.verification_codes.lock().await.codes.insert(to_mail.to_string(), code_u32);
            state.verification_codes.lock().await.created_at.insert(to_mail.to_string(), chrono::Utc::now());
            state.verification_codes.lock().await.expires_at.insert(to_mail.to_string(), chrono::Utc::now() + chrono::Duration::minutes(15));
            Ok(())
        }
        Err(e) => Err(anyhow::anyhow!("Could not send email: {e:?}"))
    }
}