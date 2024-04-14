use std::error::Error;

pub trait Repository {
    /// Map hashes from hash_array to song.
    async fn index(&self, song: &str, hash_array: &[usize]) -> Result<i32, Box<dyn Error>>;
    /// Find the most similar song by hashes.
    async fn find(&self, hash_array: &[usize]) -> Result<Option<String>, Box<dyn Error>>;
    /// Delete song from database.
    async fn delete(&self, song: &str) -> Result<u64, Box<dyn Error>>;
}

pub mod pg;