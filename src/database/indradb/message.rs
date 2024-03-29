use indradb::{Vertex, VertexQuery};
use indradb::{VertexProperty, VertexPropertyQuery};
use indradb::BulkInsertItem;

#[derive(Clone)]
pub enum Request {
    AsyncCreateVertex(Vertex),
    AsyncGetVertices(VertexQuery),
    AsyncGetVertexProperties(VertexPropertyQuery),
    AsyncSetVertexProperties(VertexPropertyQuery, serde_json::Value),
    AsyncBulkInsert(Vec<BulkInsertItem>),
}

#[derive(Clone)]
pub enum Response {
    AsyncCreateVertex(bool),
    AsyncGetVertices(Vec<Vertex>),
    AsyncGetVertexProperties(Vec<VertexProperty>),
    AsyncSetVertexProperties(()),
    AsyncBulkInsert(()),
}