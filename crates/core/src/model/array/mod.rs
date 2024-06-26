//! Type representing a Liquid array, payload of the `Value::Array` variant

use std::fmt;

use itertools::Itertools;
use itertools::Position;

use crate::model::KStringCow;

use crate::model::value::DisplayCow;
use crate::model::State;
use crate::model::Value;
use crate::model::ValueView;

/// Accessor for arrays.
pub trait ArrayView: ValueView {
    /// Cast to ValueView
    fn as_value(&self) -> &dyn ValueView;

    /// Returns the number of elements.
    fn size(&self) -> i64;

    /// Returns an iterator .
    fn values<'k>(&'k self) -> Box<dyn Iterator<Item = &'k dyn ValueView> + 'k>;

    /// Access a contained `Value`.
    fn contains_key(&self, index: i64) -> bool;
    /// Access a contained `Value`.
    fn get(&self, index: i64) -> Option<&dyn ValueView>;
    /// Returns the first element.
    fn first(&self) -> Option<&dyn ValueView> {
        self.get(0)
    }
    /// Returns the last element.
    fn last(&self) -> Option<&dyn ValueView> {
        self.get(-1)
    }
}

/// Type representing a Liquid array, payload of the `Value::Array` variant
pub type Array = Vec<Value>;

impl<T: ValueView> ValueView for Vec<T> {
    fn as_debug(&self) -> &dyn fmt::Debug {
        self
    }

    fn render(&self) -> DisplayCow<'_> {
        DisplayCow::Owned(Box::new(ArrayRender { s: self }))
    }
    fn source(&self) -> DisplayCow<'_> {
        DisplayCow::Owned(Box::new(ArraySource { s: self }))
    }
    fn type_name(&self) -> &'static str {
        "array"
    }
    fn query_state(&self, state: State) -> bool {
        match state {
            State::Truthy => true,
            State::DefaultValue | State::Empty | State::Blank => self.is_empty(),
        }
    }

    fn to_kstr(&self) -> KStringCow<'_> {
        let s = ArrayRender { s: self }.to_string();
        KStringCow::from_string(s)
    }
    fn to_value(&self) -> Value {
        let a = self.iter().map(|v| v.to_value()).collect();
        Value::Array(a)
    }

    fn as_array(&self) -> Option<&dyn ArrayView> {
        Some(self)
    }
}

impl<T: ValueView> ArrayView for Vec<T> {
    fn as_value(&self) -> &dyn ValueView {
        self
    }

    fn size(&self) -> i64 {
        self.len() as i64
    }

    fn values<'k>(&'k self) -> Box<dyn Iterator<Item = &'k dyn ValueView> + 'k> {
        let i = self.as_slice().iter().map(|v| convert_value(v));
        Box::new(i)
    }

    fn contains_key(&self, index: i64) -> bool {
        let index = convert_index(index, self.size());
        index < self.size()
    }

    fn get(&self, index: i64) -> Option<&dyn ValueView> {
        let index = convert_index(index, self.size());
        let value = self.as_slice().get(index as usize);
        value.map(|v| convert_value(v))
    }
}

impl<'a, A: ArrayView + ?Sized> ArrayView for &'a A {
    fn as_value(&self) -> &dyn ValueView {
        <A as ArrayView>::as_value(self)
    }

    fn size(&self) -> i64 {
        <A as ArrayView>::size(self)
    }

    fn values<'k>(&'k self) -> Box<dyn Iterator<Item = &'k dyn ValueView> + 'k> {
        <A as ArrayView>::values(self)
    }

    fn contains_key(&self, index: i64) -> bool {
        <A as ArrayView>::contains_key(self, index)
    }

    fn get(&self, index: i64) -> Option<&dyn ValueView> {
        <A as ArrayView>::get(self, index)
    }
}

fn convert_value(s: &dyn ValueView) -> &dyn ValueView {
    s
}

fn convert_index(index: i64, max_size: i64) -> i64 {
    if 0 <= index {
        index
    } else {
        max_size + index
    }
}

struct ArraySource<'s, T: ValueView> {
    s: &'s Vec<T>,
}

impl<'s, T: ValueView> fmt::Display for ArraySource<'s, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // write!(f, "[")?;
        // for item in self.s {
        //     write!(f, "{}, ", item.render())?;
        // }
        // write!(f, "]")?;

        write!(f, "ArraySource[")?;
        println!("[");
        for (pos, item) in self.s.iter().with_position() {
            if item.is_nil() {
                write!(f, "nil")?;
                println!("nil");
            } else if item.is_array() || item.is_object() {
                let val_rendered = item.render();
                println!("array source is_array/is_object = {}", val_rendered);
                write!(f, "{}", val_rendered)?;
            } else {
                let val_json = serde_json::to_string(&item.to_value()).unwrap();

                write!(f, "{}", val_json)?;
                let mut strng = val_json.to_string();
                if strng.len() > 100 {
                    strng = format!("{}..{}", &strng[0..50], &strng[(strng.len() - 10)..]);
                }
                println!("val_json = {}", strng);

            }
            //write!(f, "{}, ", item.render())?;
            if !(pos == Position::Last || pos == Position::Only) {
                println!(",");
                write!(f, ", ")?;
            }
        }
        write!(f, "]")?;
        println!("]");
        Ok(())
    }
}

struct ArrayRender<'s, T: ValueView> {
    s: &'s Vec<T>,
}

impl<'s, T: ValueView> fmt::Display for ArrayRender<'s, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[")?;
        for (pos, item) in self.s.iter().with_position() {
            if item.is_nil() {
                write!(f, "nil")?;
            } else if item.is_array() || item.is_object() {
                let val_rendered = item.render();
                write!(f, "{}", val_rendered)?;

                let mut strng = val_rendered.to_string();
                if strng.len() > 100 {
                    strng = format!("{}..{}", &strng[0..50], &strng[(strng.len() - 10)..]);
                }

            } else {
                let val_json = serde_json::to_string(&item.to_value()).unwrap();

                write!(f, "{}", val_json)?;
                let mut strng = val_json.to_string();
                if strng.len() > 100 {
                    strng = format!("{}..{}", &strng[0..50], &strng[(strng.len() - 10)..]);
                }
            }
            //write!(f, "{}, ", item.render())?;

            if !(pos == Position::Last || pos == Position::Only) {
                write!(f, ", ")?;
            }

        }
        write!(f, "]")?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_array() {
        let arr = Array::new();
        println!("{}", arr.source());
        let array: &dyn ArrayView = &arr;
        println!("{}", array.source());
        let view: &dyn ValueView = array.as_value();
        println!("{}", view.source());
    }
}
