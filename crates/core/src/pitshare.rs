//! The shared pool of pit stop times (opt-in). With sharing on, each stop you make is sent as
//! six numbers: the car (iRacing's car id), tyres changed, litres put in, how long the fuel took
//! and when it started, and how long you stood. Nothing about you, your session or your track
//! is sent, and nothing identifies you: no name, customer id, key or install id. The pool's
//! stops for the car you are in are downloaded (only the car id goes with the request) so the
//! pit rejoin can time a stop before you have made one in that car.

use crate::config::ConfigStore;
use crate::engine::pitservice::{ServiceModel, StopSample};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

/// The pool's service (see services/pit-data in the repository).
pub const SERVICE: &str = "https://apex-pit-data.shamzout.workers.dev";

/// What the telemetry thread and the uploader share.
#[derive(Debug, Default)]
pub struct Link {
    /// iRacing car id of the car you are in (0: none).
    pub car_id: i64,
    /// Your stops waiting to be sent (only filled with sharing on).
    pub outbox: Vec<StopSample>,
    /// The pool's model for a car, once downloaded: taken by the telemetry thread.
    pub inbox: Vec<(i64, ServiceModel)>,
}

/// What is sent for a stop: the sample's numbers, nothing else.
#[derive(Serialize)]
struct Upload<'a> {
    game: &'static str,
    stops: &'a [StopSample],
}

#[derive(Deserialize)]
struct Pool {
    stops: Vec<StopSample>,
}

/// Downloaded pool models, kept a day per car.
#[derive(Default, Serialize, Deserialize)]
struct Cache {
    cars: BTreeMap<i64, (u64, ServiceModel)>,
}

fn now_secs() -> u64 {
    SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

pub fn spawn(config: ConfigStore, data_dir: PathBuf, link: Arc<Mutex<Link>>) {
    let _ = std::thread::Builder::new().name("pitshare".into()).spawn(move || {
        let agent = crate::net::agent();
        let cache_file = data_dir.join("pit-community.json");
        let mut cache: Cache = std::fs::read_to_string(&cache_file).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        let mut asked: Option<(i64, Instant)> = None;
        let mut send_after = Instant::now();
        loop {
            std::thread::sleep(Duration::from_secs(5));
            let cfg = config.read(|c| c.pit_rejoin.clone());
            let car = link.lock().unwrap().car_id;
            // Your stops, when you have opted in: sent in a batch, kept for later if offline.
            if cfg.share && Instant::now() >= send_after {
                let batch: Vec<StopSample> = link.lock().unwrap().outbox.iter().copied().filter(StopSample::plausible).collect();
                if !batch.is_empty() {
                    let body = Upload { game: "iracing", stops: &batch };
                    match agent.post(&format!("{SERVICE}/v1/stops")).send_json(serde_json::to_value(&body).unwrap_or_default()) {
                        Ok(_) => {
                            log::info!("pit data: shared {} stop(s)", batch.len());
                            let mut l = link.lock().unwrap();
                            l.outbox.retain(|s| !batch.contains(s));
                        }
                        Err(e) => {
                            log::info!("pit data: could not share stops ({e}); trying again later");
                            send_after = Instant::now() + Duration::from_secs(600);
                        }
                    }
                }
            } else if !cfg.share {
                link.lock().unwrap().outbox.clear();
            }
            // The pool's stops for your car: from the cache when fresh, else downloaded.
            if !cfg.use_shared || car <= 0 {
                continue;
            }
            let fresh = cache.cars.get(&car).filter(|(at, _)| now_secs().saturating_sub(*at) < 86_400).map(|(_, m)| m.clone());
            if let Some(m) = fresh {
                if asked.is_none_or(|(c, _)| c != car) {
                    asked = Some((car, Instant::now()));
                    link.lock().unwrap().inbox.push((car, m));
                }
                continue;
            }
            if asked.is_some_and(|(c, at)| c == car && at.elapsed() < Duration::from_secs(3600)) {
                continue;
            }
            asked = Some((car, Instant::now()));
            let url = format!("{SERVICE}/v1/stops?game=iracing&car={car}");
            match agent.get(&url).call().ok().and_then(|r| r.into_json::<Pool>().ok()) {
                Some(pool) => {
                    let stops: Vec<StopSample> = pool.stops.into_iter().filter(|s| s.car_id == car && s.plausible()).collect();
                    let model = ServiceModel::from_samples(&stops);
                    log::info!("pit data: {} shared stop(s) for car {car}", stops.len());
                    cache.cars.insert(car, (now_secs(), model.clone()));
                    let _ = std::fs::write(&cache_file, serde_json::to_string(&cache).unwrap_or_default());
                    link.lock().unwrap().inbox.push((car, model));
                }
                None => log::info!("pit data: shared stop times not reachable"),
            }
        }
    });
}
