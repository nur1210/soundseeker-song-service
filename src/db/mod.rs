use std::error::Error;
use crate::db::pg::Fingerprint;

pub trait Repository {
    fn index(&self, song: &str, fingerprints: Vec<Fingerprint>) -> impl std::future::Future<Output=Result<(), Box<dyn Error>>> + Send;
    fn find(&self, fingerprints: Vec<Fingerprint>) -> impl std::future::Future<Output=Result<Option<String>, Box<dyn Error>>> + Send;
}

pub mod pg;
