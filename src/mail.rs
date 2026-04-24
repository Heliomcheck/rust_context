use lettre::{
    message::{Mailbox, MessageBuilder},
    transport::smtp::authentication::Credentials,
    AsyncSmtpTransport, AsyncTransport, Tokio1Executor
};
use lettre::transport::smtp::client::TlsParameters;
use anyhow::Result;
use dotenvy::dotenv;
use std::env;
use anyhow::Context;
use lettre::transport::smtp::client::Tls;
use std::sync::Arc;
use tracing::error;


pub async fn send_mail_verif_code(to_mail: &str, state: Arc<crate::AppState>) -> Result<(), anyhow::Error>{
    dotenv().ok();

    let username = env::var("SMTP_USERNAME").context("Username is not valid")?;  // using .env file to fill out variables
    let password = env::var("SMTP_PASSWORD").context("Password is not valid")?;
    let server = env::var("SMTP_SERVER").context("Server is not valid")?;
    let port = env::var("SMTP_PORT") 
        .context("Port is not valid")?
        .parse::<u16>()
        .context("Port must be a number")?;

    let username_format = username.parse::<Mailbox>()
        .context("Invalid 'from' email")
        .inspect_err(|e| error!("Failed to parse 'from' email: {}", e))?;

    let to_mail_format = to_mail.parse::<Mailbox>()
        .context("Invalid 'to' email")
        .inspect_err(|e| error!("Failed to parse 'to' email: {}", e))?;
    
    let mut store = state.verification_store.lock().await;

    if !store.can_resend(to_mail, 30) {  // cooldown 30 сек
        return Err(anyhow::anyhow!("Too many requests. Wait before retry"));
    }

    let code = store.create(to_mail, 15);

    let email = MessageBuilder::new() // creating message
        .from(username_format)
        .to(to_mail_format)
        .subject("Your verification code")
        .body(format!("Verification code {}", code)).context("Code not found")?;

    let tls_params = TlsParameters::new(server.clone()) // TLS
        .context("Failed to create TLS parameters")?;

    let creds = Credentials::new(username.to_owned(), password.to_owned());

    let mailer = AsyncSmtpTransport::<Tokio1Executor>::relay(&server)? // create mail endpoint
        .port(port) 
        .credentials(creds)
        .tls(Tls::Required(tls_params))
        .build();

    
    match mailer.send(email).await {
        Ok(_) => Ok(()),
        Err(e) => {
            error!("Failed to send email to {}: {}", to_mail, e);
            Err(anyhow::anyhow!("Could not send email: {e:?}"))
        }
    }
}