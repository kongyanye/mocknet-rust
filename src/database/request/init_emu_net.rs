use std::mem::replace;

use serde::{Deserialize, Serialize};

use crate::database::message::{Response, ResponseFuture, DatabaseMessage, Succeed, Fail};
use crate::database::errors::BackendError;
use crate::database::backend::IndradbClientBackend;
use crate::emunet::net;
use crate::algo::in_memory_graph::InMemoryGraph;

use Response::InitEmuNet as Resp;

#[derive(Deserialize, Serialize)]
pub struct VertexInfo {
    client_id: u64, // client side vertex id in the form of u64
    description: String, // a description string to hold the place
}

#[derive(Deserialize, Serialize)]
pub struct EdgeInfo {
    client_id: (u64, u64), // client side edge id in the form of (u64, u64)
    description: String, // a description string to hold the place
}

#[derive(Deserialize, Serialize)]
pub struct Vertex {
    server_uuid: uuid::Uuid, // which server this vertex is launched on
    description: String, // a description string to hold the place
}

#[derive(Deserialize, Serialize)]
pub struct Edge {
    edge_key: (uuid::Uuid, uuid::Uuid), // out-going vertex -> incoming vertex
    description: String, // a description string to hold the place
}


pub struct InitEmuNet {
    // the Uuid of the emunet node
    emunet_uuid: uuid::Uuid,
    // a list of vertexes stored as a JSON value
    vertexes_json: serde_json::Value,
    // a list of edges stored as a JSON value
    edges_json: serde_json::Value,
}

impl DatabaseMessage<Response, BackendError> for InitEmuNet {
    fn execute<'a>(&mut self, backend: &'a IndradbClientBackend) -> ResponseFuture<'a> {        
        let msg = replace(self, InitEmuNet {
            emunet_uuid: indradb::util::generate_uuid_v1(),
            vertexes_json: serde_json::to_value(()).unwrap(),
            edges_json: serde_json::to_value(()).unwrap(),
        });
        
        Box::pin(async move {
            let msg = msg;

            let convert_res = serde_json::from_value(msg.vertexes_json);
            if convert_res.is_err() {
                return Ok(Resp(Fail("invalid json format for vertexes".to_string())));
            }
            let input_vertexes: Vec<(u64, VertexInfo)> = convert_res.unwrap();
            let input_edges: Vec<((u64, u64), EdgeInfo)> = serde_json::from_value(msg.edges_json).unwrap();

            let graph: InMemoryGraph<u64, VertexInfo,EdgeInfo> = InMemoryGraph::from_vecs(input_vertexes, input_edges).unwrap();

            Ok(Resp(Succeed(())))
        })
    }
}