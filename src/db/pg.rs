use crate::db::Repository;
use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::FromRow;
use std::collections::HashMap;
use std::env;
use std::error::Error;
use tracing::info;
use serde::{Deserialize, Serialize};

#[derive(FromRow, Debug)]
struct Hash {
    hid: i32,
    hash: String,
    time: i32,
    sid: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Fingerprint {
    pub hash: String,
    pub time: usize,
}

#[derive(FromRow)]
struct Song {
    sid: i32,
    song: String,
}

// struct Candidate {
//     song_id: i32,
//     match_num: usize,
// }
// 
// struct Table {
//     /// highest number of matches among timedelta_best
//     absolute_best: usize,
//     /// highest number of matches for every timedelta
//     timedelta_best: HashMap<i32, usize>,
// }

#[derive(Debug, Clone)]
pub struct PostgresRepository {
    pool: PgPool,
}

impl PostgresRepository {
    /// Connect to postgres database
    pub async fn open() -> Result<PostgresRepository, Box<dyn Error>> {
        let url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let pool = PgPoolOptions::new().connect(&url).await?;
        Ok(PostgresRepository { pool })
    }
    pub async fn migrate(&self) -> Result<(), Box<dyn Error>> {
        sqlx::migrate!("./migrations").run(&self.pool).await?;
        Ok(())
    }
}

impl Repository for PostgresRepository {
    async fn index(&self, song: &str, fingerprints: Vec<Fingerprint>) -> Result<(), Box<dyn Error>> {
        let mut transaction = self.pool.begin().await?;
        let query = r#"INSERT INTO songs (song) VALUES ($1) RETURNING sid;"#;

        let sid: (i32, ) = sqlx::query_as(query)
            .bind(song)
            .fetch_one(&mut transaction)
            .await?;

        let query = r#"INSERT INTO hashes (hash, time, sid) VALUES ($1, $2, $3);"#;

        for Fingerprint { time, hash } in fingerprints {
            sqlx::query(query)
                .bind(hash)
                .bind(time as i32)
                .bind(sid.0)
                .execute(&mut transaction)
                .await?;
        }

        match transaction.commit().await {
            Ok(()) => Ok(()),
            Err(e) => Err(Box::new(e)),
        }
    }

    async fn find(&self, fingerprints: Vec<Fingerprint>) -> Result<Option<String>, Box<dyn Error>> {
        let pool = &self.pool;

        // let mut cnt = HashMap::<i32, Table>::new();

        let mut mapper = HashMap::<String, Vec<i32>>::new();

        for fingerprint in &fingerprints {
            if mapper.contains_key(&fingerprint.hash) {
                mapper.get_mut(&fingerprint.hash).unwrap().push(fingerprint.time as i32);
            } else {
                mapper.insert(fingerprint.hash.clone(), vec![fingerprint.time as i32]);
            }
        }

        let mut result = Vec::<(i32, i32)>::new();
        let mut dedup_hashes = HashMap::<i32, u32>::new();

        let sql = r"SELECT * FROM hashes WHERE hash=$1;";

        for Fingerprint { time, hash } in fingerprints {
            let query = sqlx::query_as::<_, Hash>(sql).bind(hash);
            let hashes = query.fetch_all(pool).await?;

            for h in hashes {
                if dedup_hashes.contains_key(&h.sid) {
                    *dedup_hashes.get_mut(&h.sid).unwrap() += 1;
                } else {
                    dedup_hashes.insert(h.sid, 1);
                }
                for song_sampled_offset in mapper.get(&h.hash).unwrap() {
                    result.push((h.sid, h.time - song_sampled_offset));
                }
            }
        }

        result.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        let sorted_result = result;

        let mut counts_map: HashMap<(i32, i32), usize> = HashMap::new();
        for res in &sorted_result {
            let key = (res.0, res.1);
            let count = counts_map.entry(key).or_insert(0);
            *count += 1;
        }
        let counts: Vec<((i32, i32), usize)> = counts_map.into_iter().collect();

        // Group by first element and find max
        let mut songs_matches_map: HashMap<i32, (i32, i32, usize)> = HashMap::new();
        for count in &counts {
            let key = count.0.0;
            let value = songs_matches_map.entry(key).or_insert((0, 0, 0));
            if count.1 > value.2 {
                *value = (count.0.0, count.0.1, count.1);
            }
        }
        let mut songs_matches: Vec<(i32, i32, usize)> = songs_matches_map.values().cloned().collect();

        // Sort by count in descending order
        songs_matches.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.1.cmp(&b.1)));
        info!("{:?}", songs_matches);

        let sql = r"SELECT * FROM songs WHERE sid=$1;";
        let query = sqlx::query_as::<_, Song>(sql);
        let song = query.bind(songs_matches[0].0).fetch_one(pool).await?;

        // let similarity = (100.0 * matchings[0].match_num as f64 / hash_array.len() as f64) as isize;
        Ok(Some(format!("{}, {:?})", song.song, songs_matches[0].2)))
    }
}
