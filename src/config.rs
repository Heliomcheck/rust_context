use std::env;


#[derive(Debug, Clone)]

pub struct Config {
    pub database_url: String,
    pub database_url_test: String,
    pub host: String,
    pub port: u16,
    pub rust_log: String,
    pub bcrypt_cost: u32,
    pub max_album_size_mb: i64,
    pub max_photos_count: i64,
    pub smtp_server: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    pub smtp_password: String,
    pub photo_cache_max_age: u32,
}


impl Config {
pub fn from_env() -> Self {
  Self {

   database_url: env::var("DATABASE_URL").expect("DATABASE_URL not set"),
   database_url_test: env::var("DATABASE_URL_TEST").unwrap_or_else(|_| "".to_string()),
   host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
   port: env::var("PORT").unwrap_or_else(|_| "8080".to_string()).parse().expect("PORT must be a number"),
   rust_log: env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()),
   bcrypt_cost: env::var("BCRYPT_COST").unwrap_or_else(|_| "12".to_string()).parse().expect("BCRYPT_COST must be a number"),
   max_album_size_mb: env::var("MAX_ALBUM_SIZE_MB").unwrap_or_else(|_| "50".to_string()).parse().expect("MAX_ALBUM_SIZE_MB must be a number"),
   max_photos_count: env::var("MAX_PHOTOS_COUNT").unwrap_or_else(|_| "100".to_string()).parse().expect("MAX_PHOTOS_COUNT must be a number"),
   smtp_server: env::var("SMTP_SERVER").expect("SMTP_SERVER not set"),
   smtp_port: env::var("SMTP_PORT").unwrap_or_else(|_| "587".to_string()).parse().expect("SMTP_PORT must be a number"),
   smtp_username: env::var("SMTP_USERNAME").expect("SMTP_USERNAME not set"),
   smtp_password: env::var("SMTP_PASSWORD").expect("SMTP_PASSWORD not set"),
   photo_cache_max_age: env::var("PHOTO_CACHE_MAX_AGE")
    .unwrap_or_else(|_| "30".to_string())
    .parse()
    .expect("PHOTO_CACHE_MAX_AGE must be a number"),
  }
 }
}
