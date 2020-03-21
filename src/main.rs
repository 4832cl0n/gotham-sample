extern crate futures;
extern crate gotham;
#[macro_use]
extern crate gotham_derive;
extern crate hyper;
extern crate mime;
extern crate r2d2;
extern crate r2d2_redis;
extern crate r2d2_postgres;
extern crate redis;
extern crate postgres;

use hyper::{Response, StatusCode};

use gotham::http::response::create_response;
use gotham::state::State;
use gotham::pipeline::new_pipeline;
use gotham::pipeline::single::single_pipeline;
use gotham::router::Router;
use gotham::router::builder::*;
use gotham::handler::HandlerFuture;
use gotham::middleware::Middleware;
use gotham::state::FromState;

use std::error::Error;
use std::env;

use r2d2_redis::RedisConnectionManager;
use redis::Commands;
use r2d2_postgres::{PostgresConnectionManager, TlsMode};

#[derive(StateData)]
pub struct ConfigMiddlewareData {
    pub redis_conn: r2d2::PooledConnection<RedisConnectionManager>,
    pub pg_conn: r2d2::PooledConnection<PostgresConnectionManager>,
}

#[derive(Clone, NewMiddleware)]
pub struct ConfigMiddleware {
    pub redis_pool: r2d2::Pool<RedisConnectionManager>,
    pub pg_pool: r2d2::Pool<PostgresConnectionManager>,
}

impl Middleware for ConfigMiddleware {
    fn call<Chain>(self, mut state: State, chain: Chain) -> Box<HandlerFuture>
    where
        Chain: FnOnce(State) -> Box<HandlerFuture>,
    {
        state.put(ConfigMiddlewareData{
            redis_conn: self.redis_pool.get().unwrap(),
            pg_conn: self.pg_pool.get().unwrap()
        });
        chain(state)
    }
}

impl std::panic::RefUnwindSafe for ConfigMiddleware {}

pub fn handler(mut state: State) -> (State, Response) {
    let content = {
        let mut data = ConfigMiddlewareData::borrow_mut_from(&mut state);
        let redis_res: String = 
            data.redis_conn.get("予定表〜①ﾊﾝｶｸだ").unwrap();
        let pg_res: String = 
            data.pg_conn.query(
                "SELECT $1::TEXT", 
                &[
                    &"予定表〜①ﾊﾝｶｸだ3".to_string(),
                ],
            ).unwrap().get(0).get(0);
        format!("{}:{}", redis_res, pg_res)
    };

    let res = create_response(
        &state,
        StatusCode::Ok,
        Some((content.into_bytes(), mime::TEXT_PLAIN_UTF_8)),
    );

    (state, res)
}

fn router(redis_uri: &str, pg_uri: &str) -> Result<Router, Box<Error>> {
    let redis_pool = r2d2::Pool::builder()
        .build(RedisConnectionManager::new(redis_uri)?)?;
    let pg_pool = r2d2::Pool::builder()
        .build(PostgresConnectionManager::new(pg_uri, TlsMode::None)?)?;
    let config_milldeware = ConfigMiddleware {
        redis_pool: redis_pool,
        pg_pool: pg_pool,
    };
    let (chain, pipelines) = single_pipeline(
        new_pipeline()
            .add(config_milldeware)
            .build()
    );

    Ok(build_router(chain, pipelines, |route| {
        // 全てのリクエストをhandlerで実行
        route.get("/").to(handler);
        route.get("*").to(handler);
    }))
}

// cargo run -- redis://localhost:6379/0 postgres://user:pass@localhost:5432/test
pub fn main() {

    let args: Vec<String> = env::args().collect();
    let addr = "127.0.0.1:7878";
    println!("Listening for requests at http://{}", addr);

    gotham::start(addr, router(&args[1], &args[2]).unwrap());
}