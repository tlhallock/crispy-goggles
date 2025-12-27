pub mod convert;
pub mod lobby;
pub mod model;

pub mod grpc {
    tonic::include_proto!("shapes");
}
