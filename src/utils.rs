use rwf::prelude::*;
use rwf::http::ToParameter;
use uuid::Uuid;

impl ToParameter for Uuid {
    fn to_parameter(s: &str) -> Result<Self, rwf::http::Error> {
        Uuid::parse_str(s).map_err(|_| rwf::http::Error::PathDecode)
    }
}