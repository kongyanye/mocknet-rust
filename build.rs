// This build script is taken from indradb/bin/build.rs
// It compiles the indradb.capnp file and genearte the 
// capnp source code in autogen crate.

extern crate capnpc;

use std::env::var;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::path::PathBuf;

fn fix(path: PathBuf) {
    let original_contents = {
        let mut file = File::open(&path)
            .unwrap_or_else(|_| panic!("Expected to be able to open the autogenerated source at {:?}", &path));

        let mut contents = String::new();

        file.read_to_string(&mut contents)
            .unwrap_or_else(|_| panic!("Expected to be able to read the autogenerated source at {:?}", &path));

        contents
    };

    let fixed_contents = original_contents.replace("crate::indradb_capnp::", "crate::autogen::");

    let mut file = File::create(&path)
        .unwrap_or_else(|_| panic!("Expected to be able to open the autogenerated source at {:?}", &path));

    file.write_all(fixed_contents.as_bytes()).unwrap_or_else(|_| {
        panic!(
            "Expected to be able to write to the autogenerated source at {:?}",
            &path
        )
    });
}


fn main() -> Result<(), Box<dyn std::error::Error>> {
    // build protobuffer code using tonic 
    tonic_build::compile_protos("rpc_proto/proto/helloworld.proto")?;

    // build capnp code using capnp
    capnpc::CompilerCommand::new()
        .file("rpc_proto/capnp/indradb.capnp")
        .run()
        .expect("Expected to be able to compile capnp schemas");

    // Maybe add the generated capnp code to OUT_DIR?
    fix(Path::new(&var("OUT_DIR").expect("Expected `OUT_DIR` environmental variable")).join("rpc_proto/capnp/indradb_capnp.rs"));

    Ok(())
}