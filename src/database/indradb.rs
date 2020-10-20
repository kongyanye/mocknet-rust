// An implementation of Indradb storage backend
use std::future::Future;
use std::task::{Context, Poll, Poll::Ready, Poll::Pending};
use std::pin::Pin;
use std::marker::Unpin;
use std::net::SocketAddr;

use futures::AsyncReadExt;
use futures::FutureExt;

use tokio::sync::mpsc::UnboundedReceiver;

use capnp_rpc::rpc_twoparty_capnp::Side;
use capnp_rpc::{twoparty, RpcSystem};

use crate::errors::Error;
use crate::autogen;
use super::message_queue;

// A request sent from the database client
enum Request {
    Ping,
}

enum Response {
    Ping(bool),
}

struct IndradbCapnpClient {
    client: autogen::service::Client,
}

impl IndradbCapnpClient {
    fn new(client: autogen::service::Client) -> Self {
        Self {client}
    }

    async fn ping(&self) -> Result<bool, Error> {
        let req = self.client.ping_request();
        let res = req.send().promise.await?;
        Ok(res.get()?.get_ready()) 
    }
}

impl IndradbCapnpClient {
    fn build_driver(self, mut queue: message_queue::Queue<Request, Response>) -> impl Future<Output = Result<(), Error>> + 'static {
        async move {
            while let Some(msg) = queue.recv().await {
                let (req, cb_tx) = msg.take_inner();
                
                match req {
                    Request::Ping => {                        
                        let resp = self.ping().await?;
                        let send_result = cb_tx.send(Response::Ping(resp));
                        if send_result.is_err() {
                            panic!("fail to send the response back to the callback channel");
                        }
                    }
                }
            }
    
            Ok(())
        }
    }
}

pub struct IndradbConnLoop {
    rpc_system_driver: Pin<Box<dyn Future<Output = Result<(), Error>> + 'static>>,
    rpc_client_driver: Pin<Box<dyn Future<Output = Result<(), Error>> + 'static>>,
}

impl Unpin for IndradbConnLoop {}

impl Future for IndradbConnLoop {
    type Output = Result<(), Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {                
        let inner_ref = self.get_mut();
        let poll1 = inner_ref.rpc_system_driver.as_mut().poll(cx);
        match poll1 {
            Ready(res) => {
                return Ready(res);
            },
            Pending => {}
        };

        let poll2 = inner_ref.rpc_client_driver.as_mut().poll(cx);
        match poll2 {
            Ready(res) => {
                return Ready(res);
            },
            Pending => {
                return Pending;
            }
        };
    }
}

pub struct IndradbClient {
    sender: message_queue::Sender<Request, Response>,
}

pub async fn new(addr: &SocketAddr) -> Result<(IndradbClient, IndradbConnLoop), Error> {
    // Make a connection
    let stream = tokio::net::TcpStream::connect(addr).await?;
    stream.set_nodelay(true)?;
 
    // create rpc_network
    let (reader, writer) = tokio_util::compat::Tokio02AsyncReadCompatExt::compat(stream).split();
    let rpc_network = Box::new(twoparty::VatNetwork::new(
        reader,
        writer,
        Side::Client,
        Default::default(),
    ));

    // create capnp_rpc_system and indradb_capnp_client
    let mut capnp_rpc_system = RpcSystem::new(rpc_network, None);
    let indradb_capnp_client = IndradbCapnpClient {
        client: capnp_rpc_system.bootstrap(Side::Server),
    };

    // create message queue
    let (sender, queue) = message_queue::create();
    
    let rpc_system_driver = async move {
        capnp_rpc_system.await.map_err(|e| {e.into()})
    };

    let rpc_client_driver = IndradbCapnpClient::build_driver(indradb_capnp_client, queue);

    let conn_loop = IndradbConnLoop {
        rpc_system_driver: Box::pin(rpc_system_driver),
        rpc_client_driver: Box::pin(rpc_client_driver)
    };

    Ok((IndradbClient{sender}, conn_loop))
}