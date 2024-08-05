#![allow(clippy::eq_op)]
use std::fmt;
use std::sync::Arc;
use serde::Deserialize;
use serde::Serialize;

use crate::model::{State, DisplayCow, KStringCow, object::{Object, ObjectView}};
use crate::ValueCow;
use crate::ValueView;


/// Liquid Drop trait
#[typetag::serde(tag = "type")]
pub trait LiquidDrop: ObjectView + Send + Sync {}

#[derive(Clone, Debug, Serialize)]
pub struct DropObject {
    #[serde(skip)]
    inner: Arc<dyn LiquidDrop>,
    #[serde(rename = "drop_type")]
    _type: String,
}

// impl Serialize for DropData {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: serde::Serializer {
//             self.inner.serialize(serializer)
//     }
// }


impl<'de> Deserialize<'de> for DropObject {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de> {
            let val = NullDrop{};
            let typename = val.type_name().to_owned();
            Ok(DropObject{ inner: Arc::new(val),  _type: typename})
    }
}

/// Wrapper to help serialization of drops
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DropWrapper {
    /// Wrapper to help serialization of drops
    DropObject(DropObject),
}


impl DropWrapper {
    /// Create new drop wrapper for a liquid drop object
    pub fn new(val: Arc<dyn LiquidDrop>) -> Self {
        let typename = val.type_name().to_owned();
        DropWrapper::DropObject(DropObject{inner: val, _type: typename})
    }
}

impl ValueView for DropWrapper {
    fn as_debug(&self) -> &dyn fmt::Debug {
        match self {
            DropWrapper::DropObject(d) => d.inner.as_debug(),
        }
    }

    fn render(&self) -> DisplayCow<'_> {
        match self {
            DropWrapper::DropObject(d) => d.inner.render(),
        }
    }

    fn source(&self) -> DisplayCow<'_> {
        match self {
            DropWrapper::DropObject(d) => d.inner.source(),
        }
    }

    fn type_name(&self) -> &'static str {
        match self {
            DropWrapper::DropObject(d) => d.inner.type_name(),
        }
    }

    fn as_object(&self) -> Option<&dyn ObjectView> {
        match self {
            DropWrapper::DropObject(d) => d.inner.as_object(),
        }
    }

    fn query_state(&self, state: State) -> bool {
        match self {
            DropWrapper::DropObject(d) => d.inner.query_state(state),
        }
    }

    fn to_kstr(&self) -> KStringCow<'_> {
        match self {
            DropWrapper::DropObject(d) => d.inner.to_kstr(),
        }
    }

    fn to_value(&self) -> crate::Value {
        match self {
            DropWrapper::DropObject(d) => d.inner.to_value(),
        }
    }
}


impl ObjectView for DropWrapper {
    fn as_value(&self) -> &dyn ValueView {
        match self {
            DropWrapper::DropObject(d) => d.inner.as_value(),
        }
    }

    fn size(&self) -> i64 {
        match self {
            DropWrapper::DropObject(d) => d.inner.size(),
        }
    }

    fn keys<'k>(&'k self) -> Box<dyn Iterator<Item = KStringCow<'k>> + 'k> {
        match self {
            DropWrapper::DropObject(d) => d.inner.keys(),
        }
    }

    fn values<'k>(&'k self) -> Box<dyn Iterator<Item = &'k dyn ValueView> + 'k> {
        match self {
            DropWrapper::DropObject(d) => d.inner.values(),
        }
    }

    fn iter<'k>(&'k self) -> Box<dyn Iterator<Item = (KStringCow<'k>, &'k dyn ValueView)> + 'k> {
        match self {
            DropWrapper::DropObject(d) => d.inner.iter(),
        }
    }

    fn contains_key(&self, index: &str) -> bool {
        match self {
            DropWrapper::DropObject(d) => d.inner.contains_key(index),
        }
    }

    fn get<'s>(&'s self, index: &str) -> Option<ValueCow<'s>> {
        match self {
            DropWrapper::DropObject(d) => d.inner.get(index),
        }
    }
}



//use lazy_static::lazy_static;

// lazy_static! {
//     static ref DESERIALIZER_MAP: std::collections::HashMap<String, &'static str> = {
//         let mut m = HashMap::new();
//         m.insert(0, "foo");
//         m.insert(1, "bar");
//         m.insert(2, "baz");
//         m
//     };
// }



#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
struct NullDrop;

impl ValueView for NullDrop {
    fn as_debug(&self) -> &dyn std::fmt::Debug {
        self
    }

    fn render(&self) -> crate::model::DisplayCow<'_> {
        crate::Value::Nil.render()
    }
    fn source(&self) -> crate::model::DisplayCow<'_> {
        crate::Value::Nil.source()
    }
    fn type_name(&self) -> &'static str {
        "null_drop"
    }
    fn query_state(&self, state: crate::model::State) -> bool {
        match state {
            crate::model::State::Truthy => true,
            crate::model::State::DefaultValue
            | crate::model::State::Empty
            | crate::model::State::Blank => false,
        }
    }

    fn to_kstr(&self) -> crate::model::KStringCow<'_> {
        crate::model::KStringCow::from_static("")
    }
    fn to_value(&self) -> crate::Value {
        crate::Value::Object(Object::new())
    }

    fn as_object(&self) -> Option<&dyn ObjectView> {
        Some(self)
    }
}

impl ObjectView for NullDrop {
    fn as_value(&self) -> &dyn ValueView {
        self
    }

    fn size(&self) -> i64 {
        0
    }

    fn keys<'k>(&'k self) -> Box<dyn Iterator<Item = crate::model::KStringCow<'k>> + 'k> {
        let keys = Vec::new().into_iter();
        Box::new(keys)
    }

    fn values<'k>(&'k self) -> Box<dyn Iterator<Item = &'k dyn ValueView> + 'k> {
        let i = Vec::new().into_iter();
        Box::new(i)
    }

    fn iter<'k>(
        &'k self,
    ) -> Box<dyn Iterator<Item = (crate::model::KStringCow<'k>, &'k dyn ValueView)> + 'k> {
        let i = Vec::new().into_iter();
        Box::new(i)
    }

    fn contains_key(&self, _index: &str) -> bool {
        false
    }

    fn get<'s>(&'s self, _index: &str) -> Option<ValueCow<'s>> {
        None
    }
}

#[typetag::serde]
impl LiquidDrop for NullDrop {}
