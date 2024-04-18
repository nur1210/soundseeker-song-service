use std::cmp::Reverse;
use std::collections::HashMap;
use std::env;
use std::error::Error;
use sqlx::{FromRow};
use sqlx::postgres::{PgPoolOptions, PgPool};
use tracing::info;
use crate::db::Repository;

#[derive(FromRow, Debug)]
struct Hash {
    hid: i32,
    hash: i64,
    time: i32,
    sid: i32,
}

#[derive(FromRow)]
struct Song {
    sid: i32,
    song: String,
}

struct Candidate {
    song_id: i32,
    match_num: usize,
}

struct Table {
    /// highest number of matches among timedelta_best
    absolute_best: usize,
    /// highest number of matches for every timedelta
    timedelta_best: HashMap<i32, usize>,
}

#[derive(Debug)]
pub struct PostgresRepository {
    pool: PgPool,
}

impl PostgresRepository {
    /// Connect to postgres database
    pub async fn open() -> Result<PostgresRepository, Box<dyn Error>> {
        let url = env::var("DB_URL").expect("DB_URL must be set");
        let pool = PgPoolOptions::new()
            .connect(&url).await?;
        Ok(PostgresRepository { pool })
    }
        // pub async fn migrate(&self) -> Result<(), Box<dyn Error>> {
        // sqlx::migrate!("./migrations").run(&self.pool).await?;
        // Ok(())
    // }
}

impl Repository for PostgresRepository {
    async fn index(&self, song: &str, hash_array: &[usize]) -> Result<i32, Box<dyn Error>> {
        let mut transaction = self.pool.begin().await?;
        let query = r#"INSERT INTO songs (song) VALUES ($1) RETURNING sid;"#;

        let sid: (i32, ) = sqlx::query_as(query)
            .bind(song)
            .fetch_one(&mut transaction)
            .await?;

        let query = r#"INSERT INTO hashes (hash, time, sid) VALUES ($1, $2, $3);"#;

        for (time, hash) in hash_array.iter().enumerate() {
            sqlx::query(query)
                .bind(*hash as i64)
                .bind(time as i32)
                .bind(sid.0)
                .execute(&mut transaction)
                .await?;
        }


        match transaction.commit().await {
            Ok(()) => Ok(sid.0),
            Err(e) => {
                Err(Box::new(e))
            }
        }
    }

    async fn find(&self, hash_array: &[usize]) -> Result<Option<String>, Box<dyn Error>> {
        let pool = &self.pool;

        let mut cnt = HashMap::<i32, Table>::new();
        let sql = r"SELECT * FROM hashes WHERE hash=$1;";

        for (t, &h) in hash_array.iter().enumerate() {
            let query = sqlx::query_as::<_, Hash>(sql).bind(h as i64);
            let hashes = query.fetch_all(pool).await?;

            for hash in hashes {
                info!("{:?}", hash);
                *cnt.entry(hash.sid)
                    .or_insert(Table {
                        absolute_best: 0,
                        timedelta_best: HashMap::new(),
                    })
                    .timedelta_best
                    .entry(hash.time - t as i32)
                    .or_insert(0) += 1;

                if cnt[&(hash.sid)].timedelta_best[&(hash.time - t as i32)]
                    > cnt[&hash.sid].absolute_best
                {
                    cnt.get_mut(&hash.sid).unwrap().absolute_best =
                        cnt[&hash.sid].timedelta_best[&(hash.time - t as i32)]
                }
            }
        }

        if cnt.is_empty() {
            return Ok(None);
        }

        let mut matchings = Vec::<Candidate>::new();
        for (song, table) in cnt {
            matchings.push(Candidate {
                song_id: song,
                match_num: table.absolute_best,
            });
        }

        matchings.sort_by_key(|a| Reverse(a.match_num));
        
        if matchings[0].match_num == 0 {
            return Ok(None)
        }

        let sql = r"SELECT * FROM songs WHERE sid=$1;";
        let query = sqlx::query_as::<_, Song>(sql);
        let song = query
            .bind(matchings[0].song_id)
            .fetch_one(pool)
            .await?;

        // let similarity = ((matchings[0].match_num as f64 / hash_array.len() as f64) * 100.00) as isize;
        Ok(Some(song.song.to_string()))
    }

    async fn delete(&self, song: &str) -> Result<u64, Box<dyn Error>> {
        todo!()
    }
}