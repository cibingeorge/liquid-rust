use crate::model::KString;
use serde::ser::Impossible;
use serde::{self, Serialize};

use crate::model::ser::{SerError, SerializeMap, SerializeStructVariant, SerializeTupleVariant};
use crate::model::value::ser::ValueSerializer;
use crate::model::DropObj;


/// Convert a `T` into `liquid_core::model::DropObj`.
pub fn to_ldrop<T>(value: &T) -> Result<DropObj, crate::error::Error>
where
    T: Serialize,
{
    value.serialize(DropObjSerializer).map_err(|e| e.into())
}

struct DropObjSerializer;

fn object_cannot_be_a_scalar() -> SerError {
    SerError::new(crate::error::Error::with_msg("DropObj cannot be a scalar."))
}

fn object_cannot_be_an_array() -> SerError {
    SerError::new(crate::error::Error::with_msg("DropObj cannot be an array."))
}

impl serde::Serializer for DropObjSerializer {
    type Ok = DropObj;
    type Error = SerError;

    type SerializeSeq = Impossible<DropObj, SerError>;
    type SerializeTuple = Impossible<DropObj, SerError>;
    type SerializeTupleStruct = Impossible<DropObj, SerError>;
    type SerializeTupleVariant = SerializeTupleVariant<DropObj>;
    type SerializeMap = SerializeMap<DropObj>;
    type SerializeStruct = SerializeMap<DropObj>;
    type SerializeStructVariant = SerializeStructVariant<DropObj>;

    #[inline]
    fn serialize_bool(self, _value: bool) -> Result<DropObj, SerError> {
        Err(object_cannot_be_a_scalar())
    }

    #[inline]
    fn serialize_i8(self, _value: i8) -> Result<DropObj, SerError> {
        Err(object_cannot_be_a_scalar())
    }

    #[inline]
    fn serialize_i16(self, _value: i16) -> Result<DropObj, SerError> {
        Err(object_cannot_be_a_scalar())
    }

    #[inline]
    fn serialize_i32(self, _value: i32) -> Result<DropObj, SerError> {
        Err(object_cannot_be_a_scalar())
    }

    fn serialize_i64(self, _value: i64) -> Result<DropObj, SerError> {
        Err(object_cannot_be_a_scalar())
    }

    #[inline]
    fn serialize_u8(self, _value: u8) -> Result<DropObj, SerError> {
        Err(object_cannot_be_a_scalar())
    }

    #[inline]
    fn serialize_u16(self, _value: u16) -> Result<DropObj, SerError> {
        Err(object_cannot_be_a_scalar())
    }

    #[inline]
    fn serialize_u32(self, _value: u32) -> Result<DropObj, SerError> {
        Err(object_cannot_be_a_scalar())
    }

    #[inline]
    fn serialize_u64(self, _value: u64) -> Result<DropObj, SerError> {
        Err(object_cannot_be_a_scalar())
    }

    #[inline]
    fn serialize_f32(self, _value: f32) -> Result<DropObj, SerError> {
        Err(object_cannot_be_a_scalar())
    }

    #[inline]
    fn serialize_f64(self, _value: f64) -> Result<DropObj, SerError> {
        Err(object_cannot_be_a_scalar())
    }

    #[inline]
    fn serialize_char(self, _value: char) -> Result<DropObj, SerError> {
        Err(object_cannot_be_a_scalar())
    }

    #[inline]
    fn serialize_str(self, _value: &str) -> Result<DropObj, SerError> {
        Err(object_cannot_be_a_scalar())
    }

    fn serialize_bytes(self, _value: &[u8]) -> Result<DropObj, SerError> {
        Err(object_cannot_be_a_scalar())
    }

    #[inline]
    fn serialize_unit(self) -> Result<DropObj, SerError> {
        Err(object_cannot_be_a_scalar())
    }

    #[inline]
    fn serialize_unit_struct(self, _name: &'static str) -> Result<DropObj, SerError> {
        Err(object_cannot_be_a_scalar())
    }

    #[inline]
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> Result<DropObj, SerError> {
        Err(object_cannot_be_a_scalar())
    }

    #[inline]
    fn serialize_newtype_struct<T: ?Sized>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<DropObj, SerError>
    where
        T: Serialize,
    {
        value.serialize(DropObjSerializer)
    }

    fn serialize_newtype_variant<T: ?Sized>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<DropObj, SerError>
    where
        T: Serialize,
    {
        let mut values = DropObj::new();
        values.insert(
            KString::from_static(variant),
            value.serialize(ValueSerializer)?,
        );
        Ok(values)
    }

    #[inline]
    fn serialize_none(self) -> Result<DropObj, SerError> {
        Err(object_cannot_be_a_scalar())
    }

    #[inline]
    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<DropObj, SerError>
    where
        T: Serialize,
    {
        value.serialize(DropObjSerializer)
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, SerError> {
        Err(object_cannot_be_an_array())
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, SerError> {
        Err(object_cannot_be_an_array())
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, SerError> {
        Err(object_cannot_be_an_array())
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, SerError> {
        Ok(SerializeTupleVariant {
            name: KString::from_static(variant),
            vec: Vec::with_capacity(len),
            other: std::marker::PhantomData,
        })
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, SerError> {
        Ok(SerializeMap::Map {
            map: DropObj::new(),
            next_key: None,
            other: std::marker::PhantomData,
        })
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, SerError> {
        self.serialize_map(Some(len))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, SerError> {
        Ok(SerializeStructVariant {
            name: KString::from_static(variant),
            map: DropObj::new(),
            other: std::marker::PhantomData,
        })
    }
}
