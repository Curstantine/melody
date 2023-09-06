use bonsaidb::core::schema::Schema;

pub mod label;
pub mod library;
pub mod person;
pub mod release;
pub mod tag;
pub mod track;

#[derive(Debug, Schema)]
#[schema(name = "default", collections = [
    label::Label,
    library::Library,
    person::Person,
    release::Release,
    tag::Tag,
    track::Track,
])]
pub struct LocalSchema;
