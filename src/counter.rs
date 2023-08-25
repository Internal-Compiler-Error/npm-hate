use color_eyre::Result;
use serde::Serialize;
use std::{
    path::Path,
    sync::atomic::{AtomicUsize, Ordering},
};
use tokio::fs::read_to_string;

#[derive(Debug, Serialize)]
pub struct Counter(AtomicUsize);

impl Counter {
    pub async fn read_from_path<P: AsRef<Path>>(path: P) -> Result<Counter> {
        let save = read_to_string(path).await?;
        // this is a very stupid thing
        let save = save.trim_end_matches('\n');
        let counter: usize = save.parse()?;

        Ok(Counter(AtomicUsize::new(counter)))
    }

    pub fn get_val(&self) -> usize {
        self.0.load(Ordering::Relaxed)
    }

    pub fn increment(&self) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }
}
