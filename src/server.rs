use std::{error::Error, pin::pin, time::Duration};

use http::{Request, Response};
use hyper::body::{Body, Incoming};
use tokio::net::{TcpListener, ToSocketAddrs};
use tower::Service;
use tracing::{error, trace};

pub struct ServerOptions;

/// starts and serves a HTTP server capable to service HTTP1 and HTTP2 for the given service
///
/// implements graceful shutdown on ctrl-c
async fn serve_http<S, B>(
    addr: impl ToSocketAddrs,
    opts: ServerOptions,
    service: S,
) -> Result<(), std::io::Error>
where
    S: Service<Request<Incoming>, Response = Response<B>> + Send + 'static,
    S: Clone,
    S::Future: 'static + Send,
    S::Error: Into<Box<dyn Error + Send + Sync>>,
    B: Body + 'static + Send,
    B::Error: Into<Box<dyn Error + Send + Sync>> + Send,
    B::Data: Send,
{
    let listener = TcpListener::bind(addr).await?;
    let graceful = hyper_util::server::graceful::GracefulShutdown::new();
    let mut ctrl_c = pin!(tokio::signal::ctrl_c());
    let service = hyper_util::service::TowerToHyperService::new(service);

    loop {
        let watcher = graceful.watcher();
        let service = service.clone();
        let server =
            hyper_util::server::conn::auto::Builder::new(hyper_util::rt::TokioExecutor::new());

        tokio::select! {
            conn = listener.accept() => {
                let (stream, peer_addr) = match conn {
                    Ok(conn) => conn,
                    Err(e) => {
                        error!(e=%e, "accept error");
                        continue;
                    }
                };
                trace!(addr=%peer_addr, "incoming connection accepted");

                tokio::spawn(async move {
                    let conn = server.serve_connection(hyper_util::rt::TokioIo::new(stream), service);
                    let conn = watcher.watch(conn);

                    if let Err(err) = conn.await {
                        error!(e=%err, "connection error");
                    }
                    trace!(addr=%peer_addr, "connection dropped");
                });
            },

            _ = ctrl_c.as_mut() => {
                drop(listener);
                trace!("Ctrl-C received, starting shutdown");
                    break;
            }
        }
    }

    tokio::select! {
        _ = graceful.shutdown() => {
            trace!("Gracefully shutdown!");
        },
        _ = tokio::time::sleep(Duration::from_secs(10)) => {
            trace!("Waited 10 seconds for graceful shutdown, aborting...");
        }
    }
    Ok(())
}
