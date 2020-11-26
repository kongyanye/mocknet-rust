// An implementation of Indradb storage backend
use std::future::Future;

use futures::AsyncReadExt;

use capnp_rpc::rpc_twoparty_capnp::Side;
use capnp_rpc::{twoparty, RpcSystem};

use uuid::Uuid;

use super::indradb_backend::{Request, Response, IndradbClientBackend};
use super::indradb_backend::build_backend_fut;

use crate::emunet::server;
use super::message_queue::{Sender, create};
use super::message_queue;
use super::IndradbClientError;
use super::errors::BackendError;

// use tokio::net::tcp::

pub struct Client {
    sender: Sender<Request, Response, BackendError>,
}

impl Clone for Client {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone()
        }
    }
}

impl Client {
    pub async fn ping(&self) -> Result<bool, IndradbClientError> {
        let req = Request::Ping;
        let res = self.sender.send(req).await?;
        match res {
            Response::Ping(flag) => Ok(flag),
            _ => panic!("invalid response")
        }
    }

    pub async fn init(&self, servers: Vec<server::ContainerServer>) -> Result<bool, IndradbClientError> {
        let req = Request::Init(servers);
        let res = self.sender.send(req).await?;
        match res {
            Response::Init(res) => Ok(res),
            _ => panic!("invalid response")
        }
    }

    pub async fn register_user(&self, user_name: &str) -> Result<bool, IndradbClientError> {
        let req = Request::RegisterUser(user_name.to_string());
        let res = self.sender.send(req).await?;
        match res {
            Response::RegisterUser(res) => Ok(res),
            _ => panic!("invalid response")
        }
    }

    pub async fn create_emu_net(&self, user: String, net: String, capacity: u32) -> Result<Uuid, IndradbClientError> {
        let req= Request::CreateEmuNet(user, net, capacity);
        let res = self.sender.send(req).await?;
        match res {
            Response::CreateEmuNet(uuid) => Ok(uuid),
            _ => panic!("invalid response")
        }
    }
}

pub struct ClientLauncher {
    conn: tokio::net::TcpStream,
}

impl ClientLauncher {
    pub async fn new(addr: &std::net::SocketAddr) -> Result<Self, IndradbClientError> {
        let conn = tokio::net::TcpStream::connect(&addr).await.unwrap();
        Ok(Self {conn})
    }

    pub async fn with_db_client<Func, Fut>(self, entry_fn: Func) -> Result<(), IndradbClientError> 
        where
            Func: Fn(Client) -> Fut,
            Fut: Future<Output = Result<(), IndradbClientError>> + 'static + Send, 
    {
        let ls = tokio::task::LocalSet::new();
        let (sender, queue) = message_queue::create();
        
        let backend_fut = ls.run_until(async move {         
            // create rpc_system
            let (reader, writer) = tokio_util::compat::Tokio02AsyncReadCompatExt::compat(self.conn).split();
            let rpc_network = Box::new(twoparty::VatNetwork::new(
                reader,
                writer,
                Side::Client,
                Default::default(),
            ));
            let mut capnp_rpc_system = RpcSystem::new(rpc_network, None);
            
            // create client_backend
            let indradb_capnp_client = capnp_rpc_system.bootstrap(Side::Server);
            let disconnector = capnp_rpc_system.get_disconnector();
            let indradb_client_backend = IndradbClientBackend::new(indradb_capnp_client, disconnector);
    
            // run rpc_system
            tokio::task::spawn_local(async move {
                capnp_rpc_system.await
            });
            // run indradb backend
            tokio::task::spawn_local(build_backend_fut(indradb_client_backend, queue))
                .await
                .unwrap()
                .map_err(|e|{IndradbClientError::from_error(e)})
        });

        let client = Client{sender};
        let entry_fn_jh = tokio::spawn(entry_fn(client));

        backend_fut.await.unwrap();

        Ok(entry_fn_jh.await.unwrap().unwrap())
    }
}


pub fn build_client_fut<'a>(stream: tokio::net::TcpStream, ls: &'a tokio::task::LocalSet) 
    -> (Client, impl Future<Output = Result<(), IndradbClientError>> + 'a)
{
    
    let (sender, queue) = create();   

    let backend_fut = ls.run_until(async move {         
        // create rpc_system
        let (reader, writer) = tokio_util::compat::Tokio02AsyncReadCompatExt::compat(stream).split();
        let rpc_network = Box::new(twoparty::VatNetwork::new(
            reader,
            writer,
            Side::Client,
            Default::default(),
        ));
        let mut capnp_rpc_system = RpcSystem::new(rpc_network, None);
        
        // create client_backend
        let indradb_capnp_client = capnp_rpc_system.bootstrap(Side::Server);
        let disconnector = capnp_rpc_system.get_disconnector();
        let indradb_client_backend = IndradbClientBackend::new(indradb_capnp_client, disconnector);

        // run rpc_system
        tokio::task::spawn_local(async move {
            capnp_rpc_system.await
        });
        // run indradb backend
        tokio::task::spawn_local(build_backend_fut(indradb_client_backend, queue))
            .await
            .unwrap()
            .map_err(|e|{IndradbClientError::from_error(e)})
    });
    
    (Client{sender}, backend_fut)
}