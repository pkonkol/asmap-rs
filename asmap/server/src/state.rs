use std::{net::IpAddr, sync::Arc};

use asdb::Asdb;
use governor::{DefaultKeyedRateLimiter, Quota, RateLimiter};
use nonzero_ext::*;

const SIMPLE_PER_MIN: u32 = 20; //20_000;
const SIMPLE_MAX_BURST: u32 = 10000; //250_000;
const DETAILED_PER_SEC: u32 = 1;

// type Limiter = RateLimiter<String, DashMap<String, InMemoryState>, QuantaClock, NoOpMiddleware<QuantaInstant>>;
type LimiterKey = IpAddr;

#[derive(Clone)]
pub struct ServerState {
    pub asdb: Arc<Asdb>,
    pub simple_limiter: Arc<DefaultKeyedRateLimiter<LimiterKey>>,
    pub detailed_limiter: Arc<DefaultKeyedRateLimiter<LimiterKey>>, // Arc<Limiter>,
}

impl ServerState {
    pub async fn new(conn_str: &str, db: &str) -> Self {
        let asdb = Asdb::new(conn_str, db).await.unwrap();
        // or just get rid of nonzero_ext and do NonZeroU32::new(20).unwrap();
        let simple_limiter = Arc::new(RateLimiter::<LimiterKey, _, _, _>::keyed(
            Quota::per_minute(nonzero!(SIMPLE_PER_MIN)).allow_burst(nonzero!(SIMPLE_MAX_BURST)),
        ));
        let detailed_limiter = Arc::new(RateLimiter::<LimiterKey, _, _, _>::keyed(
            Quota::per_second(nonzero!(DETAILED_PER_SEC)),
        ));
        Self {
            asdb: Arc::new(asdb),
            simple_limiter,
            detailed_limiter,
        }
    }
}
